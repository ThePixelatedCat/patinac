use hir::{Ty, TyId};
use inkwell::{
    types::FunctionType,
    values::{BasicValue, BasicValueEnum, FloatValue, IntValue, PointerValue},
};

use crate::Codegen;

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum LayoutValue<'hir, 'ctx> {
    Scalar(ScalarKind<'hir, 'ctx>, ScalarLayout<'ctx>),

    Closure(FunctionType<'ctx>, PointerValue<'ctx>),
    Record(TyId, PointerValue<'ctx>),
    Tuple(&'hir Ty, PointerValue<'ctx>),

    Zst,
}

impl<'hir, 'ctx> LayoutValue<'hir, 'ctx> {
    pub fn int<B: BasicValue<'ctx>>(size: IntSize, int: B) -> Self {
        assert!(int.as_basic_value_enum().is_int_value());
        Self::Scalar(
            ScalarKind::Int(size),
            ScalarLayout::Direct(int.as_basic_value_enum()),
        )
    }

    pub fn indirect_int<B: BasicValue<'ctx>>(size: IntSize, int: B) -> Self {
        Self::Scalar(
            ScalarKind::Int(size),
            ScalarLayout::Indirect(int.as_basic_value_enum().into_pointer_value()),
        )
    }

    pub fn float<B: BasicValue<'ctx>>(float: B) -> Self {
        assert!(float.as_basic_value_enum().is_float_value());
        Self::Scalar(
            ScalarKind::Float,
            ScalarLayout::Direct(float.as_basic_value_enum()),
        )
    }

    pub fn indirect_float<B: BasicValue<'ctx>>(float: B) -> Self {
        Self::Scalar(
            ScalarKind::Float,
            ScalarLayout::Indirect(float.as_basic_value_enum().into_pointer_value()),
        )
    }

    pub fn array<B: BasicValue<'ctx>>(elem_ty: &'hir Ty, ptr: B) -> Self {
        assert!(ptr.as_basic_value_enum().is_pointer_value());
        Self::Scalar(
            ScalarKind::Array(elem_ty),
            ScalarLayout::Direct(ptr.as_basic_value_enum()),
        )
    }

    pub fn indirect_array<B: BasicValue<'ctx>>(elem_ty: &'hir Ty, ptr: B) -> Self {
        Self::Scalar(
            ScalarKind::Array(elem_ty),
            ScalarLayout::Indirect(ptr.as_basic_value_enum().into_pointer_value()),
        )
    }

    pub fn func_ptr<B: BasicValue<'ctx>>(ty: FunctionType<'ctx>, ptr: B) -> Self {
        Self::Scalar(
            ScalarKind::FuncPtr(ty),
            ScalarLayout::Direct(ptr.as_basic_value_enum()),
        )
    }

    pub fn as_value(&self) -> BasicValueEnum<'ctx> {
        match self {
            Self::Scalar(_, ScalarLayout::Direct(value)) => *value,
            Self::Scalar(_, ScalarLayout::Indirect(ptr)) => ptr.as_basic_value_enum(),
            Self::Closure(_, ptr) | Self::Record(_, ptr) | Self::Tuple(_, ptr) => {
                ptr.as_basic_value_enum()
            }
            Self::Zst => panic!("not a value"),
        }
    }

    pub fn as_scalar(&self) -> BasicValueEnum<'ctx> {
        let Self::Scalar(_, ScalarLayout::Direct(value)) = self else {
            panic!("not a scalar")
        };
        *value
    }

    pub fn as_int(&self) -> IntValue<'ctx> {
        let Self::Scalar(ScalarKind::Int(_), ScalarLayout::Direct(int)) = self else {
            panic!("not an int")
        };
        int.into_int_value()
    }

    pub fn as_float(&self) -> FloatValue<'ctx> {
        let Self::Scalar(ScalarKind::Float, ScalarLayout::Direct(float)) = self else {
            panic!("not an float")
        };
        float.into_float_value()
    }

    pub fn as_array(&self) -> (&'hir Ty, PointerValue<'ctx>) {
        let Self::Scalar(ScalarKind::Array(elem_ty), ScalarLayout::Direct(ptr)) = self else {
            panic!("not an array")
        };
        (elem_ty, ptr.into_pointer_value())
    }

    pub fn as_record(&self) -> PointerValue<'ctx> {
        match self {
            Self::Record(_, ptr) => *ptr,
            _ => panic!("not a record"),
        }
    }

    pub fn as_pointer(&self) -> PointerValue<'ctx> {
        match self {
            Self::Scalar(_, ScalarLayout::Indirect(ptr))
            | Self::Closure(_, ptr)
            | Self::Record(_, ptr)
            | Self::Tuple(_, ptr) => *ptr,
            _ => panic!("not a pointer"),
        }
    }

    pub fn int_op<F>(lhs: Self, rhs: Self, op: F) -> Self
    where
        F: FnOnce(IntValue<'ctx>, IntValue<'ctx>) -> IntValue<'ctx>,
    {
        let Self::Scalar(ScalarKind::Int(size), ScalarLayout::Direct(lhs)) = lhs else {
            panic!("not an int")
        };
        Self::int(size, op(lhs.into_int_value(), rhs.as_int()))
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum ScalarLayout<'ctx> {
    Direct(BasicValueEnum<'ctx>),
    Indirect(PointerValue<'ctx>),
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum ScalarKind<'hir, 'ctx> {
    Int(IntSize),
    Float,
    Array(&'hir Ty),
    FuncPtr(FunctionType<'ctx>),
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum IntSize {
    Bits8,
    Bits64,
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum StorageClass {
    Zst,
    Indirect,
    Scalar,
}

impl<'ctx> Codegen<'_, '_, 'ctx> {
    pub fn storage_class(&self, ty: &Ty) -> StorageClass {
        match ty {
            Ty::Int | Ty::UInt | Ty::Byte | Ty::Float | Ty::Char | Ty::Bool | Ty::Array(_) => {
                StorageClass::Scalar
            }
            Ty::Tuple(inner) => {
                if inner.is_empty() {
                    StorageClass::Zst
                } else {
                    StorageClass::Indirect
                }
            }
            // FIXME: non-capturing functions are register
            Ty::Func(_, _) => StorageClass::Indirect,
            Ty::Named(id) => {
                if self.hir.ty_info(*id).fields.is_empty() {
                    StorageClass::Zst
                } else {
                    StorageClass::Indirect
                }
            }
        }
    }

    pub fn is_indirect(&self, ty: &Ty) -> bool {
        self.storage_class(ty) == StorageClass::Indirect
    }

    pub fn is_zst(&self, ty: &Ty) -> bool {
        self.storage_class(ty) == StorageClass::Zst
    }
}
