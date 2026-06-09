//! Provides types for handling [packages][Package] and [modules][Module], and a [function][gather_modules] to find all the modules of a package.

use std::{
    fs::{self, DirEntry, File},
    io,
    ops::Deref,
    path::{Path, PathBuf},
};

use derive_more::{Display, Error, From};

/// A whole package, comprised of one or more modules. Generic over the contents of each module.
pub struct Package<T> {
    modules: Vec<Module<T>>,
    root_index: usize,
}

impl<'this, T> IntoIterator for &'this Package<T> {
    type Item = ModuleRef<'this, T>;
    type IntoIter = Iter<'this, T>;

    fn into_iter(self) -> Self::IntoIter {
        Iter {
            package: self,
            curr_index: 0,
        }
    }
}

impl<T> Package<T> {
    /// Returns a reference to the root module of this package.
    pub const fn root(&self) -> ModuleRef<'_, T> {
        self.get_ref(self.root_index)
    }

    /// Returns an iterator over the modules of this package.
    ///
    /// The order that modules are yielded is unstable and should not be relied on.
    pub fn iter(&self) -> <&Self as IntoIterator>::IntoIter {
        self.into_iter()
    }

    /// Applies a fallible mapping function to the contents of each module, producing a new package.
    ///
    /// # Errors
    /// Returns the first error produced by the mapping function.
    pub fn map<U, E, F: FnMut(&str, T) -> Result<U, E>>(self, mut f: F) -> Result<Package<U>, E> {
        let modules = self
            .modules
            .into_iter()
            .map(|module| {
                f(&module.name, module.contents).map(|contents| Module {
                    parent: module.parent,
                    name: module.name,
                    contents,
                    children: module.children,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Package {
            modules,
            root_index: self.root_index,
        })
    }

    fn new(root: Module<T>) -> Self {
        Self {
            modules: vec![root],
            root_index: 0,
        }
    }

    fn add(&mut self, module: Module<T>) -> usize {
        self.modules.push(module);
        self.modules.len() - 1
    }

    const fn get_ref(&self, index: usize) -> ModuleRef<'_, T> {
        ModuleRef {
            package: self,
            module_index: index,
        }
    }
}

/// An iterator over the modules of a [`Package`].
///
/// See [`Package::iter()`] for more.
pub struct Iter<'pkg, T> {
    package: &'pkg Package<T>,
    curr_index: usize,
}

impl<'pkg, T> Iterator for Iter<'pkg, T> {
    type Item = ModuleRef<'pkg, T>;

    fn next(&mut self) -> Option<Self::Item> {
        (self.curr_index < self.package.modules.len()).then(|| {
            let result = self.package.get_ref(self.curr_index);
            self.curr_index += 1;
            result
        })
    }
}

/// A module, generic over it's contents.
pub struct Module<T> {
    parent: Option<usize>,
    /// The name of this module, determined by it's filename.
    pub name: String,
    /// The generic contents of this module.
    pub contents: T,
    children: Vec<usize>,
}

/// A reference to a module and it's containing package.
///
/// Allows easy traversal of a package via [`parent()`][`Self::parent()`] and [`children()`][`Self::children()`].
#[derive(Clone, Copy)]
pub struct ModuleRef<'pkg, T> {
    package: &'pkg Package<T>,
    module_index: usize,
}

impl<T> Deref for ModuleRef<'_, T> {
    type Target = Module<T>;

    fn deref(&self) -> &Self::Target {
        &self.package.modules[self.module_index]
    }
}

impl<T> ModuleRef<'_, T> {
    /// Returns a reference to this module's parent, if it has one.
    ///
    /// All modules except the root module have a parent.
    pub fn parent(&self) -> Option<Self> {
        self.parent.map(|index| self.package.get_ref(index))
    }

    /// Returns an iterator over this module's children.
    pub fn children(&self) -> impl Iterator<Item = Self> {
        self.children.iter().map(|i| self.package.get_ref(*i))
    }
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
        let root_index = package.root_index;

        let mut children = Vec::new();
        for entry in fs::read_dir(root_path)? {
            let entry = entry?;
            if should_search(&entry) {
                children.push(gather_module(entry.path(), &mut package, Some(root_index))?);
            }
        }

        package.modules[root_index].children = children;

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
    parent: Option<usize>,
) -> Result<usize, Error> {
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

        let index = package.add(Module {
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
        Ok(package.add(Module {
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
