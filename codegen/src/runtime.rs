use inkwell::{types::BasicType, values::FunctionValue};

use crate::Codegen;

impl<'ctx> Codegen<'ctx, '_> {
    pub(crate) fn printf(&self) -> FunctionValue<'ctx> {
        if let Some(func) = self.module.get_function("printf") {
            return func;
        }
        let ty = self.ctx.i32_type().fn_type(&[self.ptr_ty().into()], true);
        self.module.add_function("printf", ty, None)
    }

    pub(crate) fn malloc(&self) -> FunctionValue<'ctx> {
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

    pub(crate) fn atomic_fetch_sub_8(&self) -> FunctionValue<'ctx> {
        if let Some(func) = self.module.get_function("__atomic_fetch_sub_8") {
            return func;
        }
        let ty = self.ctx.i64_type().fn_type(
            &[
                self.ptr_ty().into(),
                self.ctx.i64_type().into(),
                self.ctx.i32_type().into(),
            ],
            false,
        );
        self.module.add_function("__atomic_fetch_sub_8", ty, None)
    }

    pub(crate) fn atomic_fetch_add_8(&self) -> FunctionValue<'ctx> {
        if let Some(func) = self.module.get_function("__atomic_fetch_add_8") {
            return func;
        }
        let ty = self.ctx.i64_type().fn_type(
            &[
                self.ptr_ty().into(),
                self.ctx.i64_type().into(),
                self.ctx.i32_type().into(),
            ],
            false,
        );
        self.module.add_function("__atomic_fetch_add_8", ty, None)
    }
}
