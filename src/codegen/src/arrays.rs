use inkwell::{
    AtomicOrdering, AtomicRMWBinOp, IntPredicate,
    intrinsics::Intrinsic,
    types::BasicType as _,
    values::{BasicValue as _, FunctionValue, PointerValue},
};

use irs::mir::Ty;

use crate::{
    CodegenState,
    layout::{self, IntSize, LayoutValue},
};

impl<'hir, 'ctx> CodegenState<'hir, 'ctx> {
    pub fn get_array_header(&self, array: PointerValue<'ctx>) -> PointerValue<'ctx> {
        let header = unsafe {
            self.builder
                .build_in_bounds_gep(
                    self.array_header_ty(),
                    array,
                    &[self.const_int(-1)],
                    "header",
                )
                .unwrap()
        };
        let is_null = self
            .builder
            .build_int_compare(IntPredicate::EQ, array, self.const_null(), "")
            .unwrap();
        self.builder
            .build_select(is_null, self.const_null(), header, "")
            .unwrap()
            .into_pointer_value()
    }

    pub fn build_index(
        &self,
        array: LayoutValue<'hir, 'ctx>,
        index: LayoutValue<'hir, 'ctx>,
    ) -> LayoutValue<'hir, 'ctx> {
        let (elem_ty, array) = array.as_array();
        let elem_ptr = self
            .build_call(
                self.array_index(elem_ty),
                &[array.into(), index.as_int().into()],
            )
            .unwrap()
            .into_pointer_value();
        self.layout_indirect(elem_ty, elem_ptr)
    }

    pub fn array_index(&self, elem_ty: &Ty) -> FunctionValue<'ctx> {
        let func_name = format!("{}.index", self.mangle_array_ty(elem_ty));

        // Check if we already built this function
        if let Some(func) = self.module.get_function(&func_name) {
            return func;
        }

        // Save the builder's current insertion block to restore at the end
        let old_insert_block = self.builder.get_insert_block().unwrap();

        // Create the function
        let ty = self
            .ptr_ty()
            .fn_type(&[self.array_ty().into(), self.ctx.i64_type().into()], false);
        let func = self.add_func(&func_name, ty, false);
        let entry_block = self.ctx.append_basic_block(func, "entry");
        let bounds_block = self.ctx.append_basic_block(func, "bounds");
        let panic_block = self.ctx.append_basic_block(func, "panic");
        let ret_block = self.ctx.append_basic_block(func, "return");
        let array = func.get_nth_param(0).unwrap().into_pointer_value();
        let index = func.get_nth_param(1).unwrap().into_int_value();

        // Always out of bounds if the array is unallocated.
        let header = {
            self.builder.position_at_end(entry_block);
            let header = self.get_array_header(array);
            let is_null = self
                .builder
                .build_int_compare(IntPredicate::EQ, header, self.const_null(), "")
                .unwrap();
            self.builder
                .build_conditional_branch(is_null, panic_block, bounds_block)
                .unwrap();
            header
        };

        // Check if index is less than array length.
        {
            self.builder.position_at_end(bounds_block);
            let count = self
                .builder
                .build_struct_gep(self.array_header_ty(), header, 1, "")
                .unwrap();
            let count = self
                .builder
                .build_load(self.ctx.i64_type(), count, "")
                .unwrap()
                .into_int_value();
            let is_in_bounds = self
                .builder
                .build_int_compare(IntPredicate::ULT, index, count, "")
                .unwrap();
            self.builder
                .build_conditional_branch(is_in_bounds, ret_block, panic_block)
                .unwrap();
        }

        // OOB panic.
        {
            self.builder.position_at_end(panic_block);
            self.build_panic("index out of bounds");
        }

        // GEP the element and return.
        {
            self.builder.position_at_end(ret_block);
            let ptr = unsafe {
                self.builder
                    .build_in_bounds_gep(self.lower_ty(elem_ty), array, &[index], "")
                    .unwrap()
            };
            self.builder.build_return(Some(&ptr)).unwrap();
        }

        self.builder.position_at_end(old_insert_block);

        func
    }

    pub fn array_incr_refc(&self) -> FunctionValue<'ctx> {
        let func_name = "A.incr";

        // Check if we already built this function
        if let Some(func) = self.module.get_function(func_name) {
            return func;
        }

        // Save the builder's current insertion block to restore at the end
        let old_insert_block = self.builder.get_insert_block().unwrap();

        // Create the copy function
        let func_ty = self
            .ctx
            .void_type()
            .fn_type(&[self.array_ty().into()], false);
        let func = self.add_func(func_name, func_ty, false);
        let entry_block = self.ctx.append_basic_block(func, "entry");
        let incr_block = self.ctx.append_basic_block(func, "incr");
        let ret_block = self.ctx.append_basic_block(func, "return");
        let array = func.get_first_param().unwrap().into_pointer_value();

        // Return immediately if the array is unallocated.
        let header = {
            self.builder.position_at_end(entry_block);
            let header = self.get_array_header(array);
            let is_null = self
                .builder
                .build_int_compare(IntPredicate::EQ, header, self.const_null(), "")
                .unwrap();
            self.builder
                .build_conditional_branch(is_null, ret_block, incr_block)
                .unwrap();
            header
        };

        // Increment the refcount.
        {
            self.builder.position_at_end(incr_block);
            let refc = self
                .builder
                .build_struct_gep(self.array_header_ty(), header, 0, "")
                .unwrap();
            self.builder
                .build_atomicrmw(
                    AtomicRMWBinOp::Add,
                    refc,
                    self.const_int(1),
                    AtomicOrdering::Monotonic,
                )
                .unwrap();
            self.builder.build_unconditional_branch(ret_block).unwrap();
        }

        // Return.
        {
            self.builder.position_at_end(ret_block);
            self.builder.build_return(None).unwrap();
        }

        self.builder.position_at_end(old_insert_block);

        func
    }

    pub(crate) fn array_new(&self, elem_ty: &Ty) -> FunctionValue<'ctx> {
        let func_name = format!("{}.init", self.mangle_array_ty(elem_ty));

        // Check if we already built this function
        if let Some(func) = self.module.get_function(&func_name) {
            return func;
        }

        // Save the builder's current insertion block to restore at the end
        let old_insert_block = self.builder.get_insert_block().unwrap();

        // Create the init function
        let func_ty = self.ptr_ty().fn_type(&[self.ctx.i64_type().into()], false);
        let func = self.add_func(&func_name, func_ty, false);
        let entry_block = self.ctx.append_basic_block(func, "entry");
        let init_block = self.ctx.append_basic_block(func, "init");
        let ret_block = self.ctx.append_basic_block(func, "return");
        let count = func.get_first_param().unwrap().into_int_value();

        // Check if the count is zero, if so directly return a null pointer.
        {
            self.builder.position_at_end(entry_block);
            let cmp = self
                .builder
                .build_int_compare(
                    IntPredicate::EQ,
                    count,
                    self.ctx.i64_type().const_zero(),
                    "",
                )
                .unwrap();
            self.builder
                .build_conditional_branch(cmp, ret_block, init_block)
                .unwrap();
        }

        // If the count isn't zero, allocate space for capacity + header and initialise metadata.
        let payload = {
            self.builder.position_at_end(init_block);
            // Calculate capacity.
            let cap = self
                .build_call(self.array_calc_cap(elem_ty), &[count.into()])
                .unwrap()
                .into_int_value();
            let header_ty = self.array_header_ty();
            // Add room for header.
            let alloc_size = self
                .builder
                .build_int_add(cap, header_ty.size_of().unwrap(), "")
                .unwrap();
            // Allocate.
            let alloc = self
                .build_call(self.malloc(), &[alloc_size.into()])
                .unwrap()
                .into_pointer_value();
            // Initialise each field of the header.
            let refc_ptr = self
                .builder
                .build_struct_gep(header_ty, alloc, 0, "")
                .unwrap();
            self.builder
                .build_store(refc_ptr, self.const_int(1))
                .unwrap()
                .set_atomic_ordering(AtomicOrdering::Monotonic)
                .unwrap();
            let count_ptr = self
                .builder
                .build_struct_gep(header_ty, alloc, 1, "")
                .unwrap();
            self.builder.build_store(count_ptr, count).unwrap();
            let cap_ptr = self
                .builder
                .build_struct_gep(header_ty, alloc, 2, "")
                .unwrap();
            self.builder.build_store(cap_ptr, cap).unwrap();
            let payload = unsafe {
                self.builder
                    .build_in_bounds_gep(header_ty, alloc, &[self.const_int(1)], "")
                    .unwrap()
            };
            self.builder.build_unconditional_branch(ret_block).unwrap();
            payload
        };

        // Return.
        {
            self.builder.position_at_end(ret_block);
            let phi = self.builder.build_phi(self.ptr_ty(), "").unwrap();
            phi.add_incoming(&[(&self.const_null(), entry_block), (&payload, init_block)]);
            self.builder
                .build_return(Some(&phi.as_basic_value()))
                .unwrap();
        }

        self.builder.position_at_end(old_insert_block);

        func
    }

    pub(crate) fn array_unique(&self, elem_ty: &Ty) -> FunctionValue<'ctx> {
        let func_name = format!("{}.unique", self.mangle_array_ty(elem_ty));

        // Check if we already built this function
        if let Some(func) = self.module.get_function(&func_name) {
            return func;
        }

        // Save the builder's current insertion block to restore at the end.
        let old_insert_block = self.builder.get_insert_block().unwrap();

        // Create the function and blocks, and extract the arguments.
        let func_ty = self.ctx.void_type().fn_type(&[self.ptr_ty().into()], false);
        let func = self.add_func(&func_name, func_ty, false);
        let entry_block = self.ctx.append_basic_block(func, "entry");
        let unique_block = self.ctx.append_basic_block(func, "unique");
        let alloc_block = self.ctx.append_basic_block(func, "alloc");
        let store_block = self.ctx.append_basic_block(func, "store");
        let ret_block = self.ctx.append_basic_block(func, "return");
        let array = func.get_first_param().unwrap().into_pointer_value();

        // Return immediately if the array hasn't been allocated.
        // Also set up a stack allocation for later.
        let (header, index) = {
            self.builder.position_at_end(entry_block);
            let index = self
                .builder
                .build_alloca(self.ctx.i64_type(), "index")
                .unwrap();

            let header = self.get_array_header(array);
            let is_null = self
                .builder
                .build_int_compare(IntPredicate::EQ, header, self.const_null(), "")
                .unwrap();
            self.builder
                .build_conditional_branch(is_null, ret_block, unique_block)
                .unwrap();
            (header, index)
        };

        let header_ty = self.array_header_ty();

        // Check if it's already unique.
        {
            self.builder.position_at_end(unique_block);
            let refc = self
                .builder
                .build_struct_gep(header_ty, header, 0, "")
                .unwrap();
            let refc = self
                .builder
                .build_load(self.ctx.i64_type(), refc, "")
                .unwrap();
            refc.as_instruction_value()
                .unwrap()
                .set_atomic_ordering(AtomicOrdering::Acquire)
                .unwrap();
            let is_unique = self
                .builder
                .build_int_compare(
                    IntPredicate::EQ,
                    refc.into_int_value(),
                    self.const_int(1),
                    "",
                )
                .unwrap();
            self.builder
                .build_conditional_branch(is_unique, ret_block, alloc_block)
                .unwrap();
        }

        // Allocate the new array and store the metadata.
        let new_array = {
            self.builder.position_at_end(alloc_block);
            // Allocate capacity + header size.
            let capacity = self
                .builder
                .build_struct_gep(header_ty, header, 2, "")
                .unwrap();
            let capacity = self
                .builder
                .build_load(self.ctx.i64_type(), capacity, "")
                .unwrap()
                .into_int_value();
            let alloc_size = self
                .builder
                .build_int_add(capacity, header_ty.size_of().unwrap(), "")
                .unwrap();
            let alloc = self
                .build_call(self.malloc(), &[alloc_size.into()])
                .unwrap()
                .into_pointer_value();
            // Initialise refcount to 1.
            let refc_ptr = self
                .builder
                .build_struct_gep(header_ty, alloc, 0, "")
                .unwrap();
            self.builder
                .build_store(refc_ptr, self.const_int(1))
                .unwrap()
                .set_atomic_ordering(AtomicOrdering::Monotonic)
                .unwrap();
            // Initialise count to existing count.
            let count = self
                .builder
                .build_struct_gep(header_ty, header, 1, "")
                .unwrap();
            let count = self
                .builder
                .build_load(self.ctx.i64_type(), count, "")
                .unwrap()
                .into_int_value();
            let new_count = self
                .builder
                .build_struct_gep(header_ty, alloc, 1, "")
                .unwrap();
            self.builder.build_store(new_count, count).unwrap();
            // Initialise capacity to existing capacity.
            let new_capacity = self
                .builder
                .build_struct_gep(header_ty, alloc, 2, "")
                .unwrap();
            self.builder.build_store(new_capacity, capacity).unwrap();
            // Get payload of new array.
            let new_array = unsafe {
                self.builder
                    .build_in_bounds_gep(header_ty, alloc, &[self.const_int(1)], "")
                    .unwrap()
            };
            // Either memcpy or copy each element, depending on whether the element type is trivial
            if layout::trivial(elem_ty) {
                let align = self
                    .target
                    .get_target_data()
                    .get_abi_alignment(&self.lower_ty(elem_ty));
                self.builder
                    .build_memcpy(new_array, align, array, align, capacity)
                    .unwrap();
                self.builder
                    .build_unconditional_branch(store_block)
                    .unwrap();
            } else {
                let empty = self
                    .builder
                    .build_int_compare(IntPredicate::EQ, count, self.const_int(0), "")
                    .unwrap();
                let loop_block = self.ctx.prepend_basic_block(store_block, "loop");
                self.builder.build_store(index, self.const_int(0)).unwrap();
                self.builder
                    .build_conditional_branch(empty, store_block, loop_block)
                    .unwrap();
                // Loop over each element and copy it.
                {
                    self.builder.position_at_end(loop_block);
                    let curr_index = self
                        .builder
                        .build_load(self.ctx.i64_type(), index, "")
                        .unwrap()
                        .into_int_value();
                    let curr_index_val = LayoutValue::int(IntSize::Bits64, curr_index);
                    let elem = self.build_index(LayoutValue::array(elem_ty, array), curr_index_val);
                    let new_elem =
                        self.build_index(LayoutValue::array(elem_ty, new_array), curr_index_val);
                    self.build_copy(elem, new_elem.as_pointer());
                    let new_index = self
                        .builder
                        .build_int_add(curr_index, self.const_int(1), "")
                        .unwrap();
                    self.builder.build_store(index, new_index).unwrap();
                    let done = self
                        .builder
                        .build_int_compare(IntPredicate::UGE, new_index, count, "")
                        .unwrap();
                    self.builder
                        .build_conditional_branch(done, store_block, loop_block)
                        .unwrap();
                }
            }
            new_array
        };

        // Store the new payload in the array and decrement the refcount on the original array.
        {
            self.builder.position_at_end(store_block);
            self.builder.build_store(array, new_array).unwrap();
            let refc = self
                .builder
                .build_struct_gep(header_ty, header, 0, "")
                .unwrap();
            self.builder
                .build_atomicrmw(
                    AtomicRMWBinOp::Sub,
                    refc,
                    self.const_int(1),
                    AtomicOrdering::SequentiallyConsistent,
                )
                .unwrap();
            self.builder.build_unconditional_branch(ret_block).unwrap();
        }

        // Return.
        {
            self.builder.position_at_end(ret_block);
            self.builder.build_return(None).unwrap();
        }

        self.builder.position_at_end(old_insert_block);

        func
    }

    pub(crate) fn array_calc_cap(&self, elem_ty: &Ty) -> FunctionValue<'ctx> {
        let func_name = format!("{}.calc_cap", self.mangle_array_ty(elem_ty));

        // Check if we already built this function
        if let Some(func) = self.module.get_function(&func_name) {
            return func;
        }

        // Save the builder's current insertion block to restore at the end
        let old_insert_block = self.builder.get_insert_block().unwrap();

        // Create the function
        let ty = self
            .ctx
            .i64_type()
            .fn_type(&[self.ctx.i64_type().into()], false);
        let func = self.add_func(&func_name, ty, false);
        let entry_block = self.ctx.append_basic_block(func, "entry");
        let count_one_block = self.ctx.append_basic_block(func, "count.one");
        let size_not_one_block = self.ctx.append_basic_block(func, "size.not_one");
        let count_not_one_block = self.ctx.append_basic_block(func, "count.not_one");
        let count_small_block = self.ctx.append_basic_block(func, "count.small");
        let count_other_block = self.ctx.append_basic_block(func, "count.other");
        let count_not_pow2_block = self.ctx.append_basic_block(func, "count.not_pow2");
        let count_pow2_block = self.ctx.append_basic_block(func, "count.pow2");
        let ret_block = self.ctx.append_basic_block(func, "return");
        let count = func.get_nth_param(0).unwrap().into_int_value();
        let size = self.lower_ty(elem_ty).size_of().unwrap();

        // Go to special-case optimisation if the count is 1.
        {
            self.builder.position_at_end(entry_block);
            let cmp = self
                .builder
                .build_int_compare(IntPredicate::EQ, count, self.const_int(1), "")
                .unwrap();
            self.builder
                .build_conditional_branch(cmp, count_one_block, count_not_one_block)
                .unwrap();
        }

        // Special-case optimisation for 1-element arrays.
        {
            self.builder.position_at_end(count_one_block);
            let cmp = self
                .builder
                .build_int_compare(IntPredicate::EQ, size, self.const_int(1), "")
                .unwrap();
            // If element is 1 byte, return 8 bytes because the allocator will probably round to that anyway.
            // Otherwise branch further.
            self.builder
                .build_conditional_branch(cmp, ret_block, size_not_one_block)
                .unwrap();
        }

        // Special-case optimisation for 1-element arrays, cont.
        let size_not_one_cap = {
            self.builder.position_at_end(size_not_one_block);
            let cmp = self
                .builder
                .build_int_compare(IntPredicate::ULT, size, self.const_int(1025), "")
                .unwrap();
            let med_cap = self
                .builder
                .build_left_shift(size, self.const_int(2), "")
                .unwrap();
            // 4 is a good balance for medium-size elements.
            // For >1kb elements, just use 1 to avoid wasting too much memory.
            let cap = self.builder.build_select(cmp, med_cap, size, "").unwrap();
            self.builder.build_unconditional_branch(ret_block).unwrap();
            cap
        };

        // Branch to more special-cases.
        {
            self.builder.position_at_end(count_not_one_block);
            let cmp = self
                .builder
                .build_int_compare(IntPredicate::ULT, count, self.const_int(8), "")
                .unwrap();
            self.builder
                .build_conditional_branch(cmp, count_small_block, count_other_block)
                .unwrap();
        }

        // If count is less than 8, round it up to 8 elements.
        let count_small_cap = {
            self.builder.position_at_end(count_small_block);
            let cap = self
                .builder
                .build_left_shift(size, self.const_int(3), "")
                .unwrap();
            self.builder.build_unconditional_branch(ret_block).unwrap();
            cap
        };

        // Main path.
        {
            self.builder.position_at_end(count_other_block);
            let ctpop = Intrinsic::find("llvm.ctpop")
                .unwrap()
                .get_declaration(&self.module, &[self.ctx.i64_type().into()])
                .unwrap();
            let bit_count = self
                .build_c_call(ctpop, &[count.into()])
                .unwrap()
                .into_int_value();
            let is_pow2 = self
                .builder
                .build_int_compare(IntPredicate::ULT, bit_count, self.const_int(2), "")
                .unwrap();
            self.builder
                .build_conditional_branch(is_pow2, count_pow2_block, count_not_pow2_block)
                .unwrap();
        }

        // If count isn't a power of 2, round up to one.
        // (https://stackoverflow.com/a/466242)
        let rounded_count = {
            self.builder.position_at_end(count_not_pow2_block);
            let dec = self
                .builder
                .build_int_sub(count, self.const_int(1), "")
                .unwrap();
            let shr = self
                .builder
                .build_right_shift(dec, self.const_int(1), false, "")
                .unwrap();
            let or = self.builder.build_or(shr, dec, "").unwrap();
            let shr = self
                .builder
                .build_right_shift(or, self.const_int(2), false, "")
                .unwrap();
            let or = self.builder.build_or(shr, or, "").unwrap();
            let shr = self
                .builder
                .build_right_shift(or, self.const_int(4), false, "")
                .unwrap();
            let or = self.builder.build_or(shr, or, "").unwrap();
            let shr = self
                .builder
                .build_right_shift(or, self.const_int(8), false, "")
                .unwrap();
            let or = self.builder.build_or(shr, or, "").unwrap();
            let shr = self
                .builder
                .build_right_shift(or, self.const_int(16), false, "")
                .unwrap();
            let or = self.builder.build_or(shr, or, "").unwrap();
            let shr = self
                .builder
                .build_right_shift(or, self.const_int(32), false, "")
                .unwrap();
            let or = self.builder.build_or(shr, or, "").unwrap();
            let inc = self
                .builder
                .build_int_add(or, self.const_int(1), "")
                .unwrap();
            self.builder
                .build_unconditional_branch(count_pow2_block)
                .unwrap();
            inc
        };

        // Once count is a power of 2, multiply it by the element size to get our capacity.
        let count_other_cap = {
            self.builder.position_at_end(count_pow2_block);
            let phi = self.builder.build_phi(self.ctx.i64_type(), "").unwrap();
            phi.add_incoming(&[
                (&rounded_count, count_not_pow2_block),
                (&count, count_other_block),
            ]);
            let cap = self
                .builder
                .build_int_mul(phi.as_basic_value().into_int_value(), size, "")
                .unwrap();
            self.builder.build_unconditional_branch(ret_block).unwrap();
            cap
        };

        // Return the computed capacity.
        {
            self.builder.position_at_end(ret_block);
            let ret_val = self.builder.build_phi(self.ctx.i64_type(), "").unwrap();
            ret_val.add_incoming(&[
                (&self.const_int(8), count_one_block),
                (&size_not_one_cap, size_not_one_block),
                (&count_small_cap, count_small_block),
                (&count_other_cap, count_pow2_block),
            ]);
            self.builder
                .build_return(Some(&ret_val.as_basic_value()))
                .unwrap();
        }

        self.builder.position_at_end(old_insert_block);

        func
    }
}
