//! Runs golden tests

use std::{path::Path, process::ExitCode};

use codegen::OptLevel;
use compile::Args;

fn run_test(name: &str) -> ExitCode {
    eprintln!("Running test {:?}", name);
    let mut src_path = Path::new(file!()).parent().unwrap().to_path_buf();
    src_path.push(name);
    compile::compile(Args {
        src_path,
        opt_level: OptLevel::O0,
        llvmir: false,
        check: true,
    })
}

macro_rules! test {
    () => {};
    ($name:ident; $($tail:tt)*) => {
        #[test]
        fn $name() -> ExitCode {
            run_test(concat!(stringify!($name), ".ptn"))
        }
        test! { $($tail)* }
    };
    (fails $name:ident; $($tail:tt)*) => {
        #[test]
        fn $name() -> ExitCode {
            if run_test(concat!(stringify!($name), ".ptn")) == ExitCode::FAILURE { ExitCode::SUCCESS } else { ExitCode::FAILURE }
        }
        test! { $($tail)* }
    };
    (dir $name:ident; $($tail:tt)*) => {
        #[test]
        fn $name() -> ExitCode {
            run_test(stringify!($name))
        }
        test! { $($tail)* }
    };
    (fails dir $name:ident; $($tail:tt)*) => {
        #[test]
        fn $name() -> ExitCode {
            if run_test(stringify!($name)) == ExitCode::FAILURE { ExitCode::SUCCESS } else { ExitCode::FAILURE }
        }
        test! { $($tail)* }
    };
}

test! {
    array;
    closure;
    consts;
    fib;
    ifs;
    assocs;
    fails item_shadowing;
    loops;
    mut_mixing;
    record;
    sum;
    tuple;
    fails overlapping_places;
    unique_places;
    fails unresolved;
    zsts;
    dir opaque;
    dir opaque_err;
}
