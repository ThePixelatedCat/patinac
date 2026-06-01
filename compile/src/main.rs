use std::{fs, path::PathBuf, range::Range, time::Instant};

use argh::{FromArgs, from_env};
use yansi::Paint as _;

use codegen::{Codegen, CodegenMode, OptLevel};
use errors::{DiagnosticKind, ErrorHandler};
use parse::Parser;

use typecheck::TypeChecker;

#[derive(FromArgs)]
#[argh(description = "The compiler for Patina")]
#[allow(
    clippy::doc_paragraphs_missing_punctuation,
    reason = "Command line formatting conventions"
)]
struct Args {
    #[argh(positional)]
    src_path: PathBuf,

    #[argh(option, short = 'O', default = "OptLevel::default()")]
    /// level of optimisations to apply
    opt_level: OptLevel,

    #[argh(switch)]
    /// dump LLVM IR to stderr rather than emitting a binary
    dump: bool,
}

fn main() {
    let cli: Args = from_env();
    let src = match fs::read_to_string(&cli.src_path) {
        Ok(src) => src,
        Err(err) => {
            println!(
                "{error} {reading} {msg}",
                error = "error".bright_red().bold(),
                reading = "reading source file:".white().bold(),
                msg = err.white().bold()
            );
            return;
        }
    };

    let start = Instant::now();

    let handler_inner: &dyn Fn(&str, Range<usize>, DiagnosticKind) =
        &|msg, span, kind| print_diagnostic(kind, msg, span, &src);
    let handler = ErrorHandler::new(handler_inner);

    eprintln!("Parsing...");
    let Ok(ast) = Parser::new(&src, handler.clone()).parse() else {
        return;
    };

    eprintln!("Resolving...");
    let Ok(mut hir) = nameres::resolve(ast, handler.clone()) else {
        return;
    };

    eprintln!("Typechecking...");
    let Ok(ty_map) = TypeChecker::new(handler.clone()).type_program(&mut hir) else {
        return;
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
        handler,
        &ctx,
        cli.src_path.file_name().unwrap().to_str().unwrap(),
    )
    .codegen(cli.opt_level, mode);

    eprintln!(
        "{} in {}ms",
        "Done".bright_green(),
        start.elapsed().as_millis()
    );
}

fn print_diagnostic(kind: DiagnosticKind, msg: &str, span: Range<usize>, src: &str) {
    let line_start = src[..=span.start].rfind(['\n', '\r']).map_or(0, |i| i + 1);
    let line_end = src[span.end..]
        .find(['\n', '\r'])
        .map_or_else(|| src.len(), |pos| pos + span.end);

    let line = &src[line_start..line_end];

    let span_start = span.start - line_start;
    let span_end = span.end - line_start;

    let line_num = src[..=span.start].matches("\r\n").count();

    let kind_msg = match kind {
        DiagnosticKind::Error => "error".bright_red(),
        DiagnosticKind::Warning => "warning".yellow(),
    };

    let header = format!("{kind_msg}: {msg} ({}:{})", line_num + 1, span_start + 1);
    eprintln!(
        "{}\n{}   {line}\n    {:>span_end$}",
        header.white().wrap().bold(),
        ">".white().bold(),
        str::repeat("^", span_end - span_start).bright_red()
    );
}
