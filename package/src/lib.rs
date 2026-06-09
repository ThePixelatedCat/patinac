//! Provides types for handling [packages][Package] and [modules][Module], and a [function][gather_modules] to find all the modules of a package.

use std::{
    fs::{self, DirEntry, File},
    io,
    path::{Path, PathBuf},
};

use derive_more::{Display, Error, From};
use slotmap::{DefaultKey, SlotMap};

/// A whole package, comprised of one or more modules. Generic over the contents of each module.
pub struct Package<T> {
    modules: SlotMap<DefaultKey, Module<T>>,
    root_key: DefaultKey,
}

impl<T> Package<T> {
    /// Removes and returns the root module of this package.
    #[allow(
        clippy::missing_panics_doc,
        reason = "implementation detail, should never happen"
    )]
    pub fn take_root(&mut self) -> Module<T> {
        self.modules
            .remove(self.root_key)
            .expect("key was gotten from this slotmap")
    }

    /// Removes and returns all children of the provided module.
    #[allow(
        clippy::missing_panics_doc,
        reason = "implementation detail, should never happen"
    )]
    pub fn take_children_of(&mut self, module: &Module<T>) -> Vec<Module<T>> {
        module
            .children
            .iter()
            .map(|m| {
                self.modules
                    .remove(*m)
                    .expect("key was gotten from this slotmap")
            })
            .collect()
    }

    /// Applies a fallible mapping function to the contents of each module, producing a new package.
    ///
    /// # Errors
    /// Returns the first error produced by the mapping function.
    #[allow(
        clippy::missing_panics_doc,
        reason = "implementation detail, should never happen"
    )]
    pub fn map<U, E, F: FnMut(&str, T) -> Result<U, E>>(
        mut self,
        mut f: F,
    ) -> Result<Package<U>, E> {
        let mut modules = SlotMap::new();
        let root_key = self
            .modules
            .remove(self.root_key)
            .expect("key was gotten from this slotmap")
            .map(&mut f, &mut self, &mut modules, None)?;
        Ok(Package { modules, root_key })
    }

    fn new(root: Module<T>) -> Self {
        let mut modules = SlotMap::new();
        let root_key = modules.insert(root);
        Self { modules, root_key }
    }

    fn insert(&mut self, module: Module<T>) -> DefaultKey {
        self.modules.insert(module)
    }
}

impl<T> From<ModuleTree<T>> for Package<T> {
    fn from(value: ModuleTree<T>) -> Self {
        let mut modules = SlotMap::new();
        let root_key = from_tree_helper(value, &mut modules, None);
        Self { modules, root_key }
    }
}

fn from_tree_helper<T>(
    tree: ModuleTree<T>,
    modules: &mut SlotMap<DefaultKey, Module<T>>,
    parent: Option<DefaultKey>,
) -> DefaultKey {
    let key = modules.insert(Module {
        parent,
        name: tree.name,
        contents: tree.contents,
        children: Vec::new(),
    });
    modules[key].children = tree
        .children
        .into_iter()
        .map(|m| from_tree_helper(m, modules, Some(key)))
        .collect();
    key
}

/// A module, generic over it's contents.
pub struct Module<T> {
    parent: Option<DefaultKey>,
    /// The name of this module, determined by it's filename.
    pub name: String,
    /// The generic contents of this module.
    pub contents: T,
    children: Vec<DefaultKey>,
}

impl<T> Module<T> {
    fn map<U, E, F: FnMut(&str, T) -> Result<U, E>>(
        self,
        f: &mut F,
        old_package: &mut Package<T>,
        new_modules: &mut SlotMap<DefaultKey, Module<U>>,
        parent: Option<DefaultKey>,
    ) -> Result<DefaultKey, E> {
        let contents = f(&self.name, self.contents)?;
        let key = new_modules.insert(Module {
            parent,
            name: self.name,
            contents,
            children: Vec::new(),
        });
        new_modules[key].children = self
            .children
            .into_iter()
            .map(|m| {
                old_package
                    .modules
                    .remove(m)
                    .expect("key was gotten from this slotmap")
                    .map(f, old_package, new_modules, Some(key))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(key)
    }
}

/// A naive tree representation of a module structure. Used for constructing [`Packages`][Package] for tests.
pub struct ModuleTree<T> {
    /// The name of this module.
    pub name: String,
    /// The generic contents of this module.
    pub contents: T,
    /// The children of this module.
    pub children: Vec<Self>,
}

/// Errors that can arise from [`gather_modules()`].
#[derive(Debug, Display, Error, From)]
pub enum Error {
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
    #[display("module root {} not found", _0.display())]
    NoRoot(PathBuf),
    /// A user-specified source file doesn't have the `ptn` extension.
    #[error(ignore)]
    #[display("file {} is not a Patina source file", _0.display())]
    NotPtn(PathBuf),
    /// A file or directory has a non-unicode name.
    #[error(ignore)]
    #[display("{} has a non-unicode name", _0.display())]
    NotUnicode(PathBuf),
    /// Some other IO error occured.
    #[from]
    IO(io::Error),
}

/// Finds modules starting from a root path and gathers them into a [`Package`] of [`Files`][File].
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
#[allow(
    clippy::missing_panics_doc,
    reason = "implementation detail, should never happen"
)]
pub fn gather_modules(root_path: &Path) -> Result<Package<File>, Error> {
    let is_dir = match fs::metadata(root_path) {
        Ok(md) => md.is_dir(),
        Err(err) => {
            let err = match err.kind() {
                io::ErrorKind::NotFound => Error::NotFound(root_path.to_path_buf()),
                _ => Error::IO(err),
            };
            return Err(err);
        }
    };
    if is_dir {
        let root = match File::open(root_path.join("main.ptn")) {
            Ok(file) => file,
            Err(err) => {
                let err = match err.kind() {
                    io::ErrorKind::NotFound => Error::NoMain(root_path.to_path_buf()),
                    _ => Error::IO(err),
                };
                return Err(err);
            }
        };

        let mut package = Package::new(Module {
            parent: None,
            name: String::from("main"),
            contents: root,
            children: Vec::new(),
        });
        let root_key = package.root_key;

        let mut children = Vec::new();
        for entry in fs::read_dir(root_path)? {
            let entry = entry?;
            if should_search(&entry) {
                children.push(gather_module(entry.path(), &mut package, Some(root_key))?);
            }
        }

        package.modules[root_key].children = children;

        Ok(package)
    } else {
        let name = match root_path.file_prefix().expect("path is a file").to_str() {
            Some(str) => str.to_string(),
            None => return Err(Error::NotUnicode(root_path.to_path_buf())),
        };
        Ok(Package::new(Module {
            parent: None,
            name,
            contents: File::open(root_path).expect("path is known to be valid"),
            children: Vec::new(),
        }))
    }
}

/// # Panics
/// Panics if the provided path does not exist or ends in `..`.
fn gather_module(
    path: PathBuf,
    package: &mut Package<File>,
    parent: Option<DefaultKey>,
) -> Result<DefaultKey, Error> {
    let name = match path
        .file_prefix()
        .expect("provided path shouldn't end in `..`")
        .to_str()
    {
        Some(str) => str.to_string(),
        None => return Err(Error::NotUnicode(path)),
    };

    if fs::metadata(&path)
        .expect("provided path should exist")
        .is_dir()
    {
        let root = {
            let mut root_path = path.join(&name);
            root_path.set_extension("ptn");
            match File::open(&root_path) {
                Ok(file) => file,
                Err(err) => {
                    let err = match err.kind() {
                        io::ErrorKind::NotFound => Error::NoRoot(root_path),
                        _ => Error::IO(err),
                    };
                    return Err(err);
                }
            }
        };

        let index = package.insert(Module {
            parent,
            name,
            contents: root,
            children: Vec::new(),
        });

        let mut children = Vec::new();
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            if should_search(&entry) {
                children.push(gather_module(entry.path(), package, Some(index))?);
            }
        }

        package.modules[index].children = children;

        Ok(index)
    } else {
        Ok(package.insert(Module {
            parent,
            name,
            contents: File::open(path).expect("provided path should exist"),
            children: Vec::new(),
        }))
    }
}

fn should_search(entry: &DirEntry) -> bool {
    entry.file_type().is_ok_and(|ft| ft.is_dir())
        || entry.path().extension().is_some_and(|ext| ext == "ptn")
}
