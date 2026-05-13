pub mod exprs;
pub mod items;
pub mod patterns;
pub mod types;

#[derive(Default)]
pub struct Ast {
    pub adts: Vec<items::AdtItem>,
    pub execs: Vec<items::ExecItem>,
}
