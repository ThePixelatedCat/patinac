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
        let ty = self
            .ptr_ty()
            .fn_type(&[self.ctx.i64_type().as_basic_type_enum().into()], false);
        self.module.add_function("malloc", ty, None)
    }

    pub(crate) fn free(&self) -> FunctionValue<'ctx> {
        if let Some(func) = self.module.get_function("free") {
            return func;
        }
        let ty = self.ctx.void_type().fn_type(&[self.ptr_ty().into()], false);
        self.module.add_function("free", ty, None)
    }
}
