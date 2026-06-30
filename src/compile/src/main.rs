//! The driver for the compiler. Handles command-line arguments and stitches together the compilation phases.

use std::{
    fs::{self, DirEntry},
    io,
    path::{Path, PathBuf},
    process::ExitCode,
    range::Range,
    time::Instant,
};

use argh::{FromArgs, from_env};
use derive_more::{Display, Error, From};
use ident::Ident;
use slotmap::SecondaryMap;
use yansi::Paint as _;

use codegen::{CodegenMode, OptLevel, Target};
use errors::{DiagnosticKind, ErrorHandler, HandlerCallback};
use irs::{Module, ModuleId, Package};
use parse::Parser;

#[derive(FromArgs)]
#[argh(description = "The compiler for Patina")]
#[expect(
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
    // #[argh(option, short = 'T')]
    // /// the target platform to compile for, defaulting to the host platform
    // target: Option<Target>,
}

fn main() -> ExitCode {
    let args: Args = from_env();
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
    let (modules, module_paths) = match gather_modules(&args.src_path) {
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
        &|msg, span, module, kind| print_diagnostic(msg, span, kind, &sources[module]);
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
    let mode = if args.dump {
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

/// Errors that can arise from [`gather_modules()`].
#[derive(Debug, Display, Error, From)]
enum GatherError {
    /// A user-specified file or directory can't be found.
    #[error(ignore)]
    #[display("{} not found", _0.display())]
    NotFound(PathBuf),
    /// The `main.ptn` file can't be found at the expected location.
    #[error(ignore)]
    #[display("`main.ptn` file not found in {}", _0.display())]
    NoMain(PathBuf),
    /// The root file with the same name as the directory can't be found.
    #[error(ignore)]
    #[display("module root not found in {}", _0.display())]
    NoRoot(PathBuf),
    /// A file or directory has a non-unicode name.
    #[error(ignore)]
    #[display("{} has a non-unicode name", _0.display())]
    NotUnicode(PathBuf),
    /// Some other IO error occured.
    #[from]
    IO(io::Error),
}

/// Finds modules starting from a root path and gathers them into a [`Package`] and corresponding file paths.
///
/// The root path should either be a directory containing a `main.ptn` file, or a standalone file with the `ptn` extension.
///
/// # Errors
/// Returns an error if any of the following occur:
/// - Any file or directory encountered has a non-unicode name
/// - The root path doesn't exist
/// - The root path is a directory and doesn't contain a `main.ptn` file
/// - The root path is a file that doesn't have the `ptn` extension
/// - A module folder doesn't contain a `ptn` file with the same name as the folder
/// - An [`io::Error`] occured while traversing the filesystem
fn gather_modules(
    root_path: &Path,
) -> Result<(Package, SecondaryMap<ModuleId, PathBuf>), GatherError> {
    let is_dir = match fs::metadata(root_path) {
        Ok(md) => md.is_dir(),
        Err(err) => {
            let err = match err.kind() {
                io::ErrorKind::NotFound => GatherError::NotFound(root_path.to_path_buf()),
                _ => GatherError::IO(err),
            };
            return Err(err);
        }
    };

    let mut paths = SecondaryMap::new();

    let package = if is_dir {
        let main_path = root_path.join("main.ptn");
        if !main_path.exists() {
            return Err(GatherError::NoMain(root_path.to_path_buf()));
        }

        let mut package = Package::new(Module {
            parent: None,
            name: Ident::new("main"),
            children: Vec::new(),
        });
        let root_id = package.root();

        let mut children = Vec::new();
        for entry in fs::read_dir(root_path)? {
            let entry = entry?;
            if should_search(&entry, &main_path) {
                children.push(gather_module(
                    entry.path(),
                    &mut package,
                    &mut paths,
                    Some(root_id),
                )?);
            }
        }
        package.get_mut(root_id).children = children;

        paths.insert(root_id, main_path);
        package
    } else {
        let name = match root_path.file_prefix().expect("path is a file").to_str() {
            Some(str) => Ident::new(str),
            None => return Err(GatherError::NotUnicode(root_path.to_path_buf())),
        };
        let package = Package::new(Module {
            parent: None,
            name,
            children: Vec::new(),
        });
        paths.insert(package.root(), root_path.to_path_buf());
        package
    };

    Ok((package, paths))
}

/// # Panics
/// Panics if the provided path does not exist or ends in `..`.
fn gather_module(
    path: PathBuf,
    package: &mut Package,
    paths: &mut SecondaryMap<ModuleId, PathBuf>,
    parent: Option<ModuleId>,
) -> Result<ModuleId, GatherError> {
    let Some(name) = path
        .file_prefix()
        .expect("provided path shouldn't end in `..`")
        .to_str()
    else {
        return Err(GatherError::NotUnicode(path));
    };

    if fs::metadata(&path)
        .expect("provided path should exist")
        .is_dir()
    {
        let mut root_path = path.join(name);
        root_path.set_extension("ptn");
        if !root_path.exists() {
            return Err(GatherError::NoRoot(path));
        }

        let root_id = package.insert(Module {
            parent,
            name: Ident::new(name),
            children: Vec::new(),
        });

        let mut children = Vec::new();
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            if should_search(&entry, &root_path) {
                children.push(gather_module(entry.path(), package, paths, Some(root_id))?);
            }
        }
        package.get_mut(root_id).children = children;

        paths.insert(root_id, root_path);
        Ok(root_id)
    } else {
        let id = package.insert(Module {
            parent,
            name: Ident::new(name),
            children: Vec::new(),
        });
        paths.insert(id, path);
        Ok(id)
    }
}

fn should_search(entry: &DirEntry, parent: &Path) -> bool {
    let not_parent = entry.path() != parent;
    let right_type = entry.file_type().is_ok_and(|ft| ft.is_dir())
        || entry.path().extension().is_some_and(|ext| ext == "ptn");
    not_parent && right_type
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
