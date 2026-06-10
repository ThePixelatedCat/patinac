//! The driver for the compiler. Handles command-line arguments and stitches together the compilation phases.

use std::{fs, path::PathBuf, process::ExitCode, range::Range, time::Instant};

use argh::{FromArgs, from_env};
use package::ModuleId;
use slotmap::SecondaryMap;
use yansi::Paint as _;

use codegen_llvm::{Codegen, CodegenMode, OptLevel};
use errors::{DiagnosticKind, ErrorHandler, HandlerCallback};
use parse::Parser;

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

    eprintln!("Reading Files...");
    let modules = match package::gather_modules(&args.src_path) {
        Ok(modules) => modules,
        Err(err) => {
            eprintln!(
                "{} {} {}",
                "error".bright_red().bold(),
                "gathering package modules:".white().bold(),
                err.white().bold()
            );
            return ExitCode::FAILURE;
        }
    };

    let sources: SecondaryMap<ModuleId, String> = match modules
        .iter()
        .map(|(id, module)| {
            let src = fs::read_to_string(&module.path).map_err(|error| (error, &module.name))?;
            Ok((id, src))
        })
        .collect()
    {
        Ok(sources) => sources,
        Err((err, name)) => {
            eprintln!(
                "{error} {reading} {name}{colon} {msg}",
                error = "error".bright_red().bold(),
                reading = "reading module".white().bold(),
                colon = ":".white().bold(),
                msg = err.white().bold()
            );
            return ExitCode::FAILURE;
        }
    };

    let handler_inner: HandlerCallback =
        &|msg, span, module, kind| print_diagnostic(msg, span, kind, &sources[module]);
    let handler = ErrorHandler::new(handler_inner);

    //eprintln!("Parsing...");
    let Ok(asts) = sources
        .iter()
        .map(|(id, src)| {
            Parser::new(id, src, handler.clone())
                .parse()
                .map(|ast| (id, ast))
        })
        .collect()
    else {
        return ExitCode::FAILURE;
    };

    eprintln!("Resolving...");
    let Ok(mut hir) = nameres::resolve(&modules, asts, handler.clone()) else {
        return ExitCode::FAILURE;
    };

    eprintln!("Typechecking...");
    let Ok(ty_map) = typecheck::type_hir(&mut hir, handler.clone()) else {
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

fn print_diagnostic(msg: &str, span: Range<u32>, kind: DiagnosticKind, src: &str) {
    let start = usize::try_from(span.start).expect("why are you on 16bit");
    let end = usize::try_from(span.end).expect("why are you on 16bit");
    let line_start = src[..=start].rfind(['\n', '\r']).map_or(0, |i| i + 1);
    let line_end = src[end..]
        .find(['\n', '\r'])
        .map_or_else(|| src.len(), |pos| pos + end);

    let line = &src[line_start..line_end];

    let span_start = start - line_start;
    let span_end = end - line_start;

    let line_num = src[..=start].matches("\r\n").count();

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
