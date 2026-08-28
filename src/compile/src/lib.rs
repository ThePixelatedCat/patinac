//! The driver for the compiler. Handles command-line arguments and stitches together the compilation phases.

mod gather;

use std::{
    fmt::Write as _,
    fs::{self},
    path::PathBuf,
    process::ExitCode,
    range::Range,
    time::Instant,
};

use argh::FromArgs;
use slotmap::SecondaryMap;
use yansi::Paint as _;

use codegen::{CodegenMode, OptLevel, Target};
use errors::{ErrorHandler, HandlerCallback, Report, ReportKind};
use irs::ModuleId;
use parse::Parser;

#[derive(FromArgs)]
#[argh(description = "The compiler for Patina")]
#[expect(
    clippy::doc_paragraphs_missing_punctuation,
    reason = "Command line formatting conventions"
)]
pub struct Args {
    #[argh(positional)]
    pub src_path: PathBuf,

    #[argh(option, short = 'O', default = "OptLevel::default()")]
    /// level of optimisations to apply
    pub opt_level: OptLevel,

    #[argh(switch)]
    /// dump LLVM IR to stderr rather than emitting a binary
    pub llvmir: bool,

    #[argh(switch)]
    /// emit nothing, only checking if the compilation would succeed
    pub check: bool,
    // #[argh(option, short = 'T')]
    // /// the target platform to compile for, defaulting to the host platform
    // pub target: Option<Target>,
}

pub fn compile(args: Args) -> ExitCode {
    let Some(target) = Target::host() else {
        eprintln!(
            "{}{}",
            "error".bright_red().bold(),
            ": host platform is not a supported target".white().bold()
        );
        return ExitCode::FAILURE;
    };

    let start = Instant::now();

    eprintln!("Reading Files...");
    let (modules, module_paths) = match gather::gather_modules(&args.src_path) {
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

    let sources: SecondaryMap<ModuleId, String> = match module_paths
        .iter()
        .map(|(id, path)| {
            let src = fs::read_to_string(path).map_err(|error| (error, &modules.get(id).name))?;
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
        &|report, span, module| print_diagnostic(report, span, &sources[module]);
    let handler = ErrorHandler::new(handler_inner);

    eprintln!("Parsing...");
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
    let Ok(mut hir) = nameres::resolve(&modules, &asts, handler.clone()) else {
        return ExitCode::FAILURE;
    };

    eprintln!("Typechecking...");
    let Ok(expr_tys) = typecheck::type_hir(&mut hir, handler.clone()) else {
        return ExitCode::FAILURE;
    };

    eprintln!("Lowering...");
    let mir = lower::lower(handler.clone(), &hir, &expr_tys);

    eprintln!("Emitting...");
    let mode = if args.check {
        CodegenMode::Silent
    } else if args.llvmir {
        CodegenMode::IRDump
    } else {
        CodegenMode::Emit(args.src_path.with_extension("o"))
    };

    codegen::emit(
        &mir,
        args.opt_level,
        mode,
        target,
        &args
            .src_path
            .file_name()
            .expect("we read from the file earlier, so we know it is a file")
            .to_string_lossy(),
    );

    eprintln!(
        "{} in {}ms",
        "Done".bright_green(),
        start.elapsed().as_millis()
    );
    ExitCode::SUCCESS
}

fn print_diagnostic(report: Report, span: Range<u32>, src: &str) {
    let start = usize::try_from(span.start).expect("why are you on 16bit");
    let end = usize::try_from(span.end).expect("why are you on 16bit");
    let line_start = src[..=start]
        .rfind(['\n', '\r'])
        .map_or(0, |i| (i + 1).min(start));
    let line_end = src[end..]
        .find(['\n', '\r'])
        .map_or_else(|| src.len(), |pos| pos + end);

    let line = &src[line_start..line_end];

    let span_start = start - line_start;
    let span_end = end - line_start;

    let line_num = src[..=start].matches(['\n', '\r']).count();

    let kind_msg = match report.kind {
        ReportKind::Error => "error".bright_red(),
        ReportKind::Warning => "warning".yellow(),
    };

    let mut buffer = String::new();
    let _ = writeln!(
        buffer,
        "{kind_msg}: {} ({}:{})\n{} {line}\n  {:>span_end$}",
        report.name.white().bold(),
        line_num + 1,
        span_start + 1,
        ">".white().bold(),
        str::repeat("^", span_end - span_start).bright_red()
    );
    if let Some(label) = report.label {
        let _ = writeln!(
            buffer,
            "  {:>1$}",
            label.bright_red(),
            span_start + label.len()
        );
    }
    for note in report.notes {
        let _ = writeln!(buffer, "note: {note}");
    }
    eprintln!("{buffer}");
}
