use inkwell::{IntPredicate, module::Linkage, types::BasicType as _, values::FunctionValue};

use crate::Codegen;

impl<'ctx> Codegen<'_, 'ctx> {
    pub(crate) fn printf(&self) -> FunctionValue<'ctx> {
        if let Some(func) = self.module.get_function("printf") {
            return func;
        }
        let ty = self.ctx.i32_type().fn_type(&[self.ptr_ty().into()], true);
        self.module.add_function("printf", ty, None)
    }

    pub(crate) fn malloc(&self) -> FunctionValue<'ctx> {
        if let Some(func) = self.module.get_function("ptn_malloc") {
            return func;
        }

        // Save the builder's current insertion block to restore at the end.
        let old_insert_block = self.builder.get_insert_block().unwrap();

        let ty = self.ptr_ty().fn_type(&[self.ctx.i64_type().into()], false);
        let func = self
            .module
            .add_function("ptn_malloc", ty, Some(Linkage::Private));
        let entry_block = self.ctx.append_basic_block(func, "entry");
        let panic_block = self.ctx.append_basic_block(func, "panic");
        let ret_block = self.ctx.append_basic_block(func, "return");

        self.builder.position_at_end(entry_block);
        let ptr = self
            .builder
            .build_call(
                self.c_malloc(),
                &[func.get_first_param().unwrap().into()],
                "",
            )
            .unwrap()
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let is_null = self
            .builder
            .build_int_compare(IntPredicate::EQ, ptr, self.const_null(), "")
            .unwrap();
        self.builder
            .build_conditional_branch(is_null, panic_block, ret_block)
            .unwrap();

        self.builder.position_at_end(panic_block);
        self.emit_panic("allocation failed");

        self.builder.position_at_end(ret_block);
        self.builder.build_return(Some(&ptr)).unwrap();

        self.builder.position_at_end(old_insert_block);

        func
    }

    fn c_malloc(&self) -> FunctionValue<'ctx> {
        if let Some(func) = self.module.get_function("malloc") {
            return func;
        }
        let ty = self.ptr_ty().fn_type(&[self.ctx.i64_type().into()], false);
        self.module.add_function("malloc", ty, None)
    }

    pub(crate) fn free(&self) -> FunctionValue<'ctx> {
        if let Some(func) = self.module.get_function("free") {
            return func;
        }
        let ty = self.ctx.void_type().fn_type(&[self.ptr_ty().into()], false);
        self.module.add_function("free", ty, None)
    }

    pub(crate) fn panic(&self) -> FunctionValue<'ctx> {
        if let Some(func) = self.module.get_function("panic") {
            return func;
        }

        // Save the builder's current insertion block to restore at the end.
        let old_insert_block = self.builder.get_insert_block().unwrap();

        let ty = self.ctx.void_type().fn_type(&[self.ptr_ty().into()], false);
        let func = self
            .module
            .add_function("panic", ty, Some(Linkage::Private));
        self.builder
            .position_at_end(self.ctx.append_basic_block(func, "entry"));
        self.builder
            .build_call(self.printf(), &[func.get_first_param().unwrap().into()], "")
            .unwrap();
        self.builder
            .build_call(
                self.exit(),
                &[self.ctx.i32_type().const_int(1, false).into()],
                "",
            )
            .unwrap();
        self.builder.build_return(None).unwrap();

        self.builder.position_at_end(old_insert_block);

        func
    }

    fn exit(&self) -> FunctionValue<'ctx> {
        if let Some(func) = self.module.get_function("exit") {
            return func;
        }
        let ty = self
            .ctx
            .void_type()
            .fn_type(&[self.ctx.i32_type().into()], false);
        self.module.add_function("exit", ty, None)
    }
}
