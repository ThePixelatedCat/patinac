pub enum LayoutVal {
    Scalar(ScalarKind, Value),
    AutoBoxed(Value),
    FlatRecord(),
}
