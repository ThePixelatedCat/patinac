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
        if let Some(func) = self.module.get_function("_malloc") {
            return func;
        }
        let ty = self.ptr_ty().fn_type(&[self.ctx.i64_type().into()], false);
        self.module.add_function("_malloc", ty, None)
    }

    pub(crate) fn free(&self) -> FunctionValue<'ctx> {
        if let Some(func) = self.module.get_function("_free") {
            return func;
        }
        let ty = self.ctx.void_type().fn_type(&[self.ptr_ty().into()], false);
        self.module.add_function("_free", ty, None)
    }

    pub(crate) fn panic(&self) -> FunctionValue<'ctx> {
        if let Some(func) = self.module.get_function("_panic") {
            return func;
        }
        let ty = self.ctx.void_type().fn_type(&[self.ptr_ty().into()], false);
        self.module.add_function("_panic", ty, None)
    }

    /// `void _array_drop(Array* array, DropFn elem_drop, uint64_t elem_size)`
    pub(crate) fn runtime_array_drop(&self) -> FunctionValue<'ctx> {
        if let Some(func) = self.module.get_function("_array_drop") {
            return func;
        }
        let ty = self.ctx.void_type().fn_type(
            &[
                self.ptr_ty().into(),
                self.ptr_ty().into(),
                self.ctx.i64_type().into(),
            ],
            false,
        );
        self.module.add_function("_array_drop", ty, None)
    }

    /// `void _array_copy(Array* dst, Array* src)`
    pub(crate) fn runtime_array_copy(&self) -> FunctionValue<'ctx> {
        if let Some(func) = self.module.get_function("_array_copy") {
            return func;
        }
        let ty = self
            .ctx
            .void_type()
            .fn_type(&[self.ptr_ty().into(), self.ptr_ty().into()], false);
        self.module.add_function("_array_copy", ty, None)
    }

    /// `bool _array_equals(Array* lhs, Array* rhs, EqualFn elem_equals, uint64_t elem_size)`
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

    /// `void _array_unique(Array* array, CopyFn elem_copy, uint64_t elem_size)`
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

    /// `void _array_new(Array* array, uint64_t count, uint64_t elem_size)`
    pub(crate) fn runtime_array_new(&self) -> FunctionValue<'ctx> {
        if let Some(func) = self.module.get_function("_array_new") {
            return func;
        }
        let ty = self.ctx.bool_type().fn_type(
            &[
                self.ptr_ty().into(),
                self.ctx.i64_type().into(),
                self.ctx.i64_type().into(),
            ],
            false,
        );
        self.module.add_function("_array_new", ty, None)
    }

    /// `void _array_bounds_check(Array* array, uint64_t idx)`
    pub(crate) fn runtime_bounds_check(&self) -> FunctionValue<'ctx> {
        if let Some(func) = self.module.get_function("_array_bounds_check") {
            return func;
        }
        let ty = self
            .ctx
            .void_type()
            .fn_type(&[self.ptr_ty().into(), self.ctx.i64_type().into()], false);
        self.module.add_function("_array_bounds_check", ty, None)
    }
}
