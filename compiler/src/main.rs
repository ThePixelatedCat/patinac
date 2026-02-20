use anyhow::anyhow;
use std::{env, fs};
use string_interner::DefaultStringInterner;

use crate::parser::Parser;

mod ast;
mod helpers;
mod hir;
mod lexer;
mod parser;
mod resolver;
//mod typecheck;

//use typecheck::TypeChecker;

fn main() -> anyhow::Result<()> {
    let source_path = env::args()
        .nth(1)
        .ok_or_else(|| anyhow!("source filepath argument missing"))?;
    let source = fs::read_to_string(source_path)?;

    let mut interner = DefaultStringInterner::default();
    let mut parser = Parser::new(&source, &mut interner);

    let ast = parser.parse()?;

    //TypeChecker::new().check(&ast)?;

    Ok(())
}
