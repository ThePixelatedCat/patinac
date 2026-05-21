use std::{fmt::Display, fs, path::PathBuf, time::Instant};

use clap::Parser as CliParser;
use codegen::{Codegen, CodegenMode, OptLevel};
use yansi::Paint;

use parse::Parser;
use span::Span;

use typecheck::TypeChecker;

#[derive(CliParser)]
#[command(name = "PatinaC", version)]
#[command(about = "The compiler for Patina", long_about = None)]
struct Cli {
    src_path: PathBuf,
    #[arg(short = 'O', default_value_t)]
    opt_level: OptLevel,
    #[arg(short, long)]
    dump: bool,
}

fn main() {
    let cli = Cli::parse();
    let src = fs::read_to_string(&cli.src_path).unwrap();

    let start = Instant::now();

    eprintln!("Lexing...");
    let toks = match lex::lex(&src) {
        Ok(tokens) => tokens,
        Err(spans) => {
            for span in spans {
                print_diagnostic(DiagnosticKind::Error, "invalid token", span, &src);
            }
            return;
        }
    };

    eprintln!("Parsing...");
    let ast = match Parser::new(toks).parse() {
        Ok(ast) => ast,
        Err(errs) => {
            for err in errs {
                print_diagnostic(DiagnosticKind::Error, &err.msg(), err.span(), &src);
            }
            return;
        }
    };

    eprintln!("Resolving...");
    let mut hir = match nameres::resolve(ast) {
        Ok(v) => v,
        Err(err) => {
            print_diagnostic(DiagnosticKind::Error, &err.msg(), err.span(), &src);
            return;
        }
    };

    eprintln!("Typechecking...");
    let ty_map = match TypeChecker::default().type_program(&mut hir) {
        Ok(v) => v,
        Err(err) => {
            print_diagnostic(DiagnosticKind::Error, &err.msg(), err.span(), &src);
            return;
        }
    };

    eprintln!("Compiling...");
    let mode = if cli.dump {
        CodegenMode::IRDump
    } else {
        CodegenMode::Emit(cli.src_path.with_extension("o"))
    };
    let ctx = codegen::create_ctx();
    Codegen::new(
        &hir,
        &ty_map,
        &ctx,
        &cli.src_path.file_name().unwrap().to_str().unwrap(),
    )
    .codegen(cli.opt_level, mode);

    eprintln!(
        "{} in {}ms",
        "Done".bright_green(),
        start.elapsed().as_millis()
    );
}

#[derive(Clone, Copy)]
enum DiagnosticKind {
    Error,
    Warning,
}

impl Display for DiagnosticKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Error => "error".bright_red().fmt(f),
            Self::Warning => "warning".yellow().fmt(f),
        }
    }
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
        "    {:>span_end$}",
        str::repeat("^", span_end - span_start).bright_red(),
    );
}
