//! Gathering module files.

use std::{
    error::Error,
    fs::{self, DirEntry},
    io,
    path::{Path, PathBuf},
};

use derive_more::Display;
use ident::Ident;
use slotmap::SecondaryMap;

use irs::{Module, ModuleId, Package};

/// Errors that can arise from [`gather_modules()`].
#[derive(Debug, Display)]
pub enum GatherError {
    /// A user-specified file or directory can't be found.
    #[display("{} not found", _0.display())]
    NotFound(PathBuf),
    /// The `main.ptn` file can't be found at the expected location.
    #[display("`main.ptn` file not found in {}", _0.display())]
    NoMain(PathBuf),
    /// The root file with the same name as the directory can't be found.
    #[display("module root not found in {}", _0.display())]
    NoRoot(PathBuf),
    /// A file or directory has a non-unicode name.
    #[display("{} has a non-unicode name", _0.display())]
    NotUnicode(PathBuf),
    /// Some other IO error occured.
    IO(io::Error),
}

impl Error for GatherError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::IO(error) => Some(error),
            _ => None,
        }
    }
}

impl From<io::Error> for GatherError {
    fn from(value: io::Error) -> Self {
        Self::IO(value)
    }
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
pub fn gather_modules(
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
