//! The driver for the compiler. Handles command-line arguments and stitches together the compilation phases.

use std::process::ExitCode;

fn main() -> ExitCode {
    compile::compile(argh::from_env())
}
