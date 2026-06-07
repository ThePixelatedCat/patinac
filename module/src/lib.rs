//! Provides a [type][Module] representing a module tree, and a [function][gather_modules] to find all the modules of a package.

use std::{
    fs::{self, DirEntry, File},
    path::Path,
};

/// A tree of modules, generic over the contents of a single module.
pub struct Module<T> {
    name: String,
    contents: T,
    children: Vec<Self>,
}

impl<T> Module<T> {
    pub fn map<U, F: FnMut(T) -> U>(self, mut f: F) -> Module<U> {
        let children = self.children.into_iter().map(|m| m.map(&mut f)).collect();
        Module {
            name: self.name,
            contents: f(self.contents),
            children,
        }
    }
}

/// Gathers all the modules of a package into a [`Module`] of [`Files`][File].
pub fn gather_modules(path: &Path) -> Module<File> {
    if fs::metadata(path).unwrap().is_dir() {
        let name = path.file_name().unwrap().to_str().unwrap().to_string();

        let root = {
            let mut path = path.to_path_buf();
            path.push(&name);
            path.set_extension(".ptn");
            File::open(&path).unwrap()
        };

        let mut children = Vec::new();
        for entry in fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            if should_search(&entry) {
                children.push(gather_modules(&entry.path()));
            }
        }

        Module {
            name,
            contents: root,
            children,
        }
    } else {
        Module {
            name: path.file_prefix().unwrap().to_str().unwrap().to_string(),
            contents: File::open(path).unwrap(),
            children: vec![],
        }
    }
}

fn should_search(entry: &DirEntry) -> bool {
    entry.file_type().is_ok_and(|ft| ft.is_dir())
        || entry.path().extension().is_some_and(|ext| ext == "ptn")
}
