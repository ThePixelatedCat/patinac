//! Runs golden tests

use std::{fs, path::Path, process::ExitCode};

use compile::{Args, OptLevel};

#[test]
fn run_tests() {
    for file in fs::read_dir(Path::new(file!()).parent().unwrap()).unwrap() {
        let file = file.unwrap();
        if file.path().extension().is_some_and(|ext| ext == "ptn") {
            let args = Args {
                src_path: file.path(),
                opt_level: OptLevel::O0,
                dump: false,
            };
            assert_eq!(compile::compile(args), ExitCode::SUCCESS)
        }
    }
}
