//! Provides types for handling [packages][Package] and [modules][Module], and a [function][gather_modules] to find all the modules of a package.

use std::{
    fs::{self, DirEntry},
    io,
    path::{Path, PathBuf},
};

use derive_more::{Display, Error, From, IntoIterator};
use ident::Ident;
use slotmap::{SlotMap, new_key_type};

/// A whole package, comprised of one or more modules. Generic over the contents of each module.
#[derive(Debug, IntoIterator)]
pub struct Package {
    #[into_iterator(ref)]
    modules: SlotMap<ModuleId, Module>,
    root_id: ModuleId,
}

impl Package {
    /// Returns the id for the root module.
    pub const fn root(&self) -> ModuleId {
        self.root_id
    }

    /// Returns the module associated with the given id.
    pub fn get(&self, id: ModuleId) -> &Module {
        &self.modules[id]
    }

    /// Returns a by-reference iterator over the modules of this package.
    pub fn iter(&self) -> <&Self as IntoIterator>::IntoIter {
        self.into_iter()
    }

    fn new(root: Module) -> Self {
        let mut modules = SlotMap::default();
        let root_id = modules.insert(root);
        Self { modules, root_id }
    }

    fn insert(&mut self, module: Module) -> ModuleId {
        self.modules.insert(module)
    }
}

// impl<T> From<ModuleTree<T>> for Package<T> {
//     fn from(value: ModuleTree<T>) -> Self {
//         let mut modules = SlotMap::default();
//         let root_key = from_tree_helper(value, &mut modules, None);
//         Self { modules, root_key }
//     }
// }

// fn from_tree_helper(
//     tree: ModuleTree<T>,
//     modules: &mut SlotMap<ModuleKey, Module>,
//     parent: Option<ModuleKey>,
// ) -> ModuleKey {
//     let key = modules.insert(Module {
//         parent,
//         name: tree.name,
//         contents: tree.contents,
//         children: Vec::new(),
//     });
//     modules[key].children = tree
//         .children
//         .into_iter()
//         .map(|m| from_tree_helper(m, modules, Some(key)))
//         .collect();
//     key
// }

new_key_type! {
    /// An ID for a module.
    pub struct ModuleId;
}

/// A module, generic over it's contents.
#[derive(Debug)]
pub struct Module {
    /// The parent module of this module, unless this is the root module.
    pub parent: Option<ModuleId>,
    /// The name of this module, determined by it's filename.
    pub name: Ident,
    /// The filepath of this module.
    pub path: PathBuf,
    /// Any child modules of this module.
    pub children: Vec<ModuleId>,
}

// /// A naive tree representation of a module structure. Used for constructing [`Packages`][Package] for tests.
// pub struct ModuleTree {
//     /// The name of this module.
//     pub name: String,
//     /// The children of this module.
//     pub children: Vec<Self>,
// }

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
    #[display("module root not found in {}", _0.display())]
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

/// Finds modules starting from a root path and gathers them into a [`Package`].
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
#[expect(
    clippy::missing_panics_doc,
    reason = "implementation detail, should never happen"
)]
pub fn gather_modules(root_path: &Path) -> Result<Package, Error> {
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
        let main_path = root_path.join("main.ptn");
        if !main_path.exists() {
            return Err(Error::NoMain(root_path.to_path_buf()));
        }

        let mut package = Package::new(Module {
            parent: None,
            name: Ident::new("main"),
            path: main_path.clone(),
            children: Vec::new(),
        });
        let root_id = package.root_id;

        let mut children = Vec::new();
        for entry in fs::read_dir(root_path)? {
            let entry = entry?;
            if should_search(&entry, &main_path) {
                children.push(gather_module(entry.path(), &mut package, Some(root_id))?);
            }
        }

        package.modules[root_id].children = children;

        Ok(package)
    } else {
        let name = match root_path.file_prefix().expect("path is a file").to_str() {
            Some(str) => Ident::new(str),
            None => return Err(Error::NotUnicode(root_path.to_path_buf())),
        };
        Ok(Package::new(Module {
            parent: None,
            name,
            path: root_path.to_path_buf(),
            children: Vec::new(),
        }))
    }
}

/// # Panics
/// Panics if the provided path does not exist or ends in `..`.
fn gather_module(
    path: PathBuf,
    package: &mut Package,
    parent: Option<ModuleId>,
) -> Result<ModuleId, Error> {
    let Some(name) = path
        .file_prefix()
        .expect("provided path shouldn't end in `..`")
        .to_str()
    else {
        return Err(Error::NotUnicode(path));
    };

    if fs::metadata(&path)
        .expect("provided path should exist")
        .is_dir()
    {
        let mut root_path = path.join(name);
        root_path.set_extension("ptn");
        if !root_path.exists() {
            return Err(Error::NoRoot(path));
        }

        let root_id = package.insert(Module {
            parent,
            name: Ident::new(name),
            path: root_path.clone(),
            children: Vec::new(),
        });

        let mut children = Vec::new();
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            if should_search(&entry, &root_path) {
                children.push(gather_module(entry.path(), package, Some(root_id))?);
            }
        }

        package.modules[root_id].children = children;

        Ok(root_id)
    } else {
        Ok(package.insert(Module {
            parent,
            name: Ident::new(name),
            path,
            children: Vec::new(),
        }))
    }
}

fn should_search(entry: &DirEntry, parent: &Path) -> bool {
    let not_parent = entry.path() != parent;
    let right_type = entry.file_type().is_ok_and(|ft| ft.is_dir())
        || entry.path().extension().is_some_and(|ext| ext == "ptn");
    not_parent && right_type
}
