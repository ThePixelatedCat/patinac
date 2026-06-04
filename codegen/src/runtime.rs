use inkwell::{types::BasicType as _, values::FunctionValue};

use crate::Codegen;

impl<'ctx> Codegen<'_, '_, 'ctx> {
    pub(crate) fn printf(&self) -> FunctionValue<'ctx> {
        if let Some(func) = self.module.get_function("printf") {
            return func;
        }
        let ty = self.ctx.i32_type().fn_type(&[self.ptr_ty().into()], true);
        self.module.add_function("printf", ty, None)
    }

    pub(crate) fn malloc(&self) -> FunctionValue<'ctx> {
        if let Some(func) = self.module.get_function("_malloc") {
            return func;
        }
        let ty = self.ptr_ty().fn_type(&[self.ctx.i64_type().into()], false);
        self.module.add_function("_malloc", ty, None)
    }

    pub(crate) fn free(&self) -> FunctionValue<'ctx> {
        if let Some(func) = self.module.get_function("free") {
            return func;
        }
        let ty = self.ctx.void_type().fn_type(&[self.ptr_ty().into()], false);
        self.module.add_function("free", ty, None)
    }

    pub(crate) fn panic(&self) -> FunctionValue<'ctx> {
        if let Some(func) = self.module.get_function("_panic") {
            return func;
        }
        let ty = self.ctx.void_type().fn_type(&[self.ptr_ty().into()], false);
        self.module.add_function("_panic", ty, None)
    }

    /// `bool _array_equals(Array* lhs, Array* rhs, EqualFn elem_equals, uint64_t elem_size)`.
    pub(crate) fn runtime_array_equals(&self) -> FunctionValue<'ctx> {
        if let Some(func) = self.module.get_function("_array_equals") {
            return func;
        }
        let ty = self.ctx.bool_type().fn_type(
            &[
                self.ptr_ty().into(),
                self.ptr_ty().into(),
                self.ptr_ty().into(),
                self.ctx.i64_type().into(),
            ],
            false,
        );
        self.module.add_function("_array_equals", ty, None)
    }

    /// `void _array_unique(Array* array, CopyFn elem_copy, uint64_t elem_size)`.
    pub(crate) fn runtime_array_unique(&self) -> FunctionValue<'ctx> {
        if let Some(func) = self.module.get_function("_array_unique") {
            return func;
        }
        let ty = self.ctx.bool_type().fn_type(
            &[
                self.ptr_ty().into(),
                self.ptr_ty().into(),
                self.ctx.i64_type().into(),
            ],
            false,
        );
        self.module.add_function("_array_unique", ty, None)
    }
}
