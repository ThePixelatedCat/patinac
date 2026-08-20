//! Runs golden tests

use std::{fs, path::Path, process::ExitCode};

use codegen::OptLevel;
use compile::Args;

#[test]
fn run_tests() {
    for file in fs::read_dir(Path::new(file!()).parent().unwrap()).unwrap() {
        let file = file.unwrap();
        if file.path().extension().is_some_and(|ext| ext == "ptn") {
            eprintln!("Running test {:?}", file.path().file_name().unwrap());
            let args = Args {
                src_path: file.path(),
                opt_level: OptLevel::O0,
                llvmir: false,
                check: true,
            };
            let exit_code = if Path::new(file.path().file_stem().unwrap())
                .extension()
                .is_some_and(|ext| ext == "e")
            {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            };
            assert_eq!(compile::compile(args), exit_code)
        }
    }
}
