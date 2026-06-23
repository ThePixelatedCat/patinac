use inkwell::{
    AtomicOrdering, AtomicRMWBinOp, IntPredicate,
    module::Linkage,
    types::BasicType as _,
    values::{FunctionValue, PointerValue},
};

use mir::Ty;

use crate::{CodegenState, layout::LayoutValue};

impl<'hir, 'ctx> CodegenState<'hir, 'ctx> {
    pub fn get_array_header(&self, array: PointerValue<'ctx>) -> PointerValue<'ctx> {
        let header = unsafe {
            self.builder
                .build_in_bounds_gep(
                    self.array_header_ty(),
                    array,
                    &[self.ctx.i64_type().const_int(1, true).const_neg()],
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

    pub fn emit_array_indexing(
        &self,
        array: LayoutValue<'hir, 'ctx>,
        index: LayoutValue<'hir, 'ctx>,
    ) -> LayoutValue<'hir, 'ctx> {
        let (elem_ty, array) = array.as_array();
        let elem_ptr = self
            .builder
            .build_call(
                self.array_index(elem_ty),
                &[array.into(), index.as_int().into()],
                "",
            )
            .unwrap()
            .try_as_basic_value()
            .unwrap_basic()
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
        let func = self
            .module
            .add_function(&func_name, ty, Some(Linkage::Private));
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
            self.emit_panic("index out of bounds");
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
        let func = self
            .module
            .add_function(func_name, func_ty, Some(Linkage::Private));
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
                    self.ctx.i64_type().const_int(1, false),
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
}
