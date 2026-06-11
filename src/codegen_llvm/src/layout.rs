use inkwell::values::{BasicValue, BasicValueEnum, PointerValue};

#[derive(Debug, Clone, Copy)]
pub enum LayoutValue<'ctx> {
    Scalar(BasicValueEnum<'ctx>),
    Indirect(PointerValue<'ctx>),
    Unit,
}

impl<'ctx> LayoutValue<'ctx> {
    pub fn scalar<V: BasicValue<'ctx>>(scalar: V) -> Self {
        Self::Scalar(scalar.as_basic_value_enum())
    }

    pub fn as_scalar(self) -> BasicValueEnum<'ctx> {
        match self {
            Self::Scalar(value) => value,
            _ => panic!("not a scalar"),
        }
    }

    /// Return either a scalar value or an indirect pointer.
    pub fn as_value(self) -> BasicValueEnum<'ctx> {
        match self {
            Self::Scalar(value) => value,
            Self::Indirect(pointer) => pointer.as_basic_value_enum(),
            _ => panic!("not a value"),
        }
    }
}

impl<'ctx, V: BasicValue<'ctx>> From<V> for LayoutValue<'ctx> {
    fn from(value: V) -> Self {
        Self::scalar(value)
    }
}
