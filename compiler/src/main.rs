use anyhow::anyhow;
use std::{env, fs};

use crate::parser::Parser;

mod helpers;
mod lexer;
mod parser;
mod typecheck;

fn main() -> anyhow::Result<()> {
    let source_path = env::args()
        .nth(1)
        .ok_or_else(|| anyhow!("source filepath argument missing"))?;
    let source = fs::read_to_string(source_path)?;

    let mut parser = Parser::new(&source);

    let ast = parser.file()?;
    println!("{ast:?}");

    typecheck::TypeChecker::new(&ast).type_of(&parser::ast::Expr::Int(42).spanned(0..2))?;

    Ok(())
}
