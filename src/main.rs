use anyhow::anyhow;
use std::{env, fs};

mod lexer;
mod parser;

use lexer::Lexer;

fn main() -> anyhow::Result<()> {
    let source_path = env::args()
        .nth(1)
        .ok_or(anyhow!("source filepath argument missing"))?;
    let source = fs::read_to_string(source_path)?;

    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize();

    let ast = parser::parse(tokens);

    Ok(())
}
