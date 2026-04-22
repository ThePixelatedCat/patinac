use std::{env, fmt::Display, fs};

use anyhow::anyhow;
use yansi::Paint;

use lex::Lexer;
use parse::Parser;
use span::Span;

//use typecheck::TypeChecker;

enum DiagnosticKind {
    Error,
    Warning,
}

impl Display for DiagnosticKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiagnosticKind::Error => "error".bright_red().fmt(f),
            DiagnosticKind::Warning => "warning".yellow().fmt(f),
        }
    }
}

fn main() {
    let src_path = env::args()
        .nth(1)
        .ok_or_else(|| anyhow!("source filepath argument missing"))
        .unwrap();
    let src = fs::read_to_string(src_path).unwrap();

    let toks = match Lexer::lex(&src) {
        Ok(tokens) => tokens,
        Err(spans) => {
            for span in spans {
                print_diagnostic(DiagnosticKind::Error, "invalid token", span, &src);
            }
            return;
        }
    };

    let ast = match Parser::parse(toks) {
        Ok(ast) => ast,
        Err(errs) => {
            for err in errs {
                print_diagnostic(DiagnosticKind::Error, &err.kind.to_string(), err.span, &src);
            }
            return;
        }
    };

    //TypeChecker::new().check(&ast)?;
}

fn print_diagnostic(kind: DiagnosticKind, msg: &str, span: Span, src: &str) {
    let line_start = src[..=span.start].rfind(['\n', '\r']).map_or(0, |i| i + 1);
    let line_end = src[span.end..]
        .find(['\n', '\r'])
        .map_or_else(|| src.len(), |pos| pos + span.end);

    let line = &src[line_start..line_end];

    let span_start = span.start - line_start;
    let span_end = span.end - line_start;

    let line_num = src[..=span.start].matches("\r\n").count();

    let header = format!("{kind}: {msg} ({}:{})", line_num + 1, span_start + 1);
    println!("{}", header.white().wrap().bold());
    println!("{}   {line}", ">".white().bold());
    println!(
        "    {:>span_end$} {}",
        str::repeat("^", span_end - span_start).bright_red(),
        ""
    );
}
