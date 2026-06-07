//! The driver for the compiler. Handles command-line arguments and stitches together the compilation phases.

use std::{fs, path::PathBuf, process::ExitCode, range::Range, time::Instant};

use argh::{FromArgs, from_env};
use yansi::Paint as _;

use codegen_llvm::{Codegen, CodegenMode, OptLevel};
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

fn main() -> ExitCode {
    let args: Args = from_env();

    let start = Instant::now();

    let handler_inner: &dyn Fn(&str, Range<usize>, DiagnosticKind) =
        &|msg, span, kind| print_diagnostic(msg, span, kind, &src);
    let handler = ErrorHandler::new(handler_inner);

    eprintln!("Traversing...");
    let modules = module::gather_modules(&args.src_path);

    eprintln!("Parsing...");
    let src = match fs::read_to_string(&args.src_path) {
        Ok(src) => src,
        Err(err) => {
            eprintln!(
                "{error} {reading} {msg}",
                error = "error".bright_red().bold(),
                reading = "reading source file:".white().bold(),
                msg = err.white().bold()
            );
            return ExitCode::FAILURE;
        }
    };

    let Ok(ast) = Parser::new(&src, handler.clone()).parse() else {
        return ExitCode::FAILURE;
    };

    eprintln!("Resolving...");
    let Ok(mut hir) = nameres::resolve(ast, handler.clone()) else {
        return ExitCode::FAILURE;
    };

    eprintln!("Typechecking...");
    let Ok(ty_map) = TypeChecker::new(handler.clone()).type_program(&mut hir) else {
        return ExitCode::FAILURE;
    };

    eprintln!("Compiling...");
    let mode = if args.dump {
        CodegenMode::IRDump
    } else {
        CodegenMode::Emit(args.src_path.with_extension("o"))
    };
    let ctx = codegen_llvm::create_ctx();
    Codegen::new(
        &hir,
        &ty_map,
        handler,
        &ctx,
        &args
            .src_path
            .file_name()
            .expect("we read from the file earlier, so we know it is a file")
            .to_string_lossy(),
    )
    .codegen(args.opt_level, mode);

    eprintln!(
        "{} in {}ms",
        "Done".bright_green(),
        start.elapsed().as_millis()
    );
    ExitCode::SUCCESS
}

fn print_diagnostic(msg: &str, span: Range<usize>, kind: DiagnosticKind, src: &str) {
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
