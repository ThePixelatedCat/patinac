use std::{env, fs};

use anyhow::anyhow;
use yansi::Paint;

use lex::Lexer;
use parse::Parser;
use span::Span;

//use typecheck::TypeChecker;

fn main() {
    let src_path = env::args()
        .nth(1)
        .ok_or_else(|| anyhow!("source filepath argument missing"))
        .unwrap();
    let src = fs::read_to_string(src_path).unwrap();

    let tokens = match Lexer::lex(&src) {
        Ok(tokens) => tokens,
        Err(spans) => {
            for span in spans {
                print_diagnostic("invalid token", span, &src);
            }
            return;
        }
    };

    let mut parser = Parser::new(&src, tokens.into_iter().peekable());

    let ast = match parser.parse() {
        Ok(ast) => ast,
        Err(errs) => {
            for err in errs {
                print_diagnostic(&err.0.to_string(), err.1, &src);
            }
            return;
        }
    };

    //TypeChecker::new().check(&ast)?;
}

fn print_diagnostic(msg: &str, span: Span, src: &str) {
    let line_start = src[..=span.start].rfind(['\n', '\r']).map_or(0, |i| i + 1);
    let line_end = src[span.end..]
        .find(['\n', '\r'])
        .map_or_else(|| src.len(), |pos| pos + span.end);

    let line = &src[line_start..line_end];

    let span_start = span.start - line_start;
    let span_end = span.end - line_start;

    let header = format!("{}: {msg} ({}:{span_start})", "error".bright_red(), 0);
    println!("{}", header.bold());
    println!("{line}");
    println!(
        "{:>span_end$} {}",
        str::repeat("^", span_end - span_start).bright_red(),
        ""
    );
}
