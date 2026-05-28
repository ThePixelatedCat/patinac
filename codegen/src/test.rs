use errors::TEST_HANDLER;
use parse::Parser;
use typecheck::TypeChecker;

use crate::{Codegen, CodegenMode, OptLevel};

fn check(input: &str, opt_level: OptLevel) {
    let ast = Parser::new(input, TEST_HANDLER).parse().unwrap();
    let mut hir = nameres::resolve(ast, TEST_HANDLER).unwrap();
    let ty_map = TypeChecker::new(TEST_HANDLER)
        .type_program(&mut hir)
        .unwrap();
    let ctx = crate::create_ctx();
    Codegen::new(&hir, &ty_map, &ctx, "test").codegen(opt_level, CodegenMode::Silent);
}

#[test]
fn weird_sum() {
    let input = "
    fn sum(n: Int, m: Int): Int = {
        let mut foo = n
        foo = foo + 1
        foo + {m - 1}
    }
";
    check(input, OptLevel::O0);
}

#[test]
fn ifs() {
    let input = "
    fn foo(n: Float): Float = 
        if n >= 0.0 { n } else { 0.0 } *. 2.0
";
    check(input, OptLevel::O0);

    let input = "
    fn foo(n: Float): Float = {
        let mut m = n
        if n < 0.0 { m = 0.0 }
        m *. 2.0
    }
";
    check(input, OptLevel::O0);
}

#[test]
fn nested_if() {
    let input = "
    fn foo(a: Bool, b: Bool): Float = {   
        let mut out = 0.0
        if a { 
            if b { 
                out = 2.0 
            } else { 
                out = 5.0 
            } 
        }
        out
    }
";
    check(input, OptLevel::O0);
}

#[test]
fn call() {
    let input = "
    fn sum(n: Int, m: Int): Int = n + m
    fn inc(n: Int): Int = sum(n, 1)
";
    check(input, OptLevel::O0);
}

#[test]
fn fib() {
    let input = "
    fn fib(n: Float): Float =
        if n < 2.0 { n } else { fib(n -. 1.0) +. fib(n -. 2.0) }
";
    check(input, OptLevel::O0);
}

#[test]
fn mut_loop() {
    let input = "
    fn looping_inc(mut n: Int): () = 
        loop {
            n = n + 1
        }
";
    check(input, OptLevel::O0);
}

#[test]
fn mut_arg() {
    let input = "
    fn inc(mut n: Int): () = n = n + 1
    fn do_inc(): () = {
        let mut m = 5
        inc(mut m)
    }
";
    check(input, OptLevel::O0);
}

#[test]
fn record_field() {
    let input = "
    record Point(x: Float, y: Float)
    fn get_x(self: Point): Float = self.x
    fn make_point(x: Float): Point = Point(x, 0.0)
    fn main(): () = {
        let point = make_point(1.0)
        print get_x(point)
    }
";
    check(input, OptLevel::O0);
}

#[test]
fn record_equals() {
    let input = "
    record Point(x: Float, y: Float)
    fn main(): () = {
        print Point(1.0, 1.0) == Point(1.0, 1.0)
        print Point(2.0, 3.0) == Point(3.0, 2.0)
    }
";
    check(input, OptLevel::O0);
}

#[test]
fn tuples() {
    let input = "
    fn main(): () = {
        let foo = (1)
        let bar = if false {}
        let baz = tupleception(foo)
    }   

    fn tupleception(v: (UInt)): ((UInt), (UInt)) = (v, v)
";
    check(input, OptLevel::O0);
}

#[test]
fn closures() {
    let input = "
    fn main(): () = {
        let m = 3
        apply(fn(n) -> print n + m, 2)
    }

    fn apply(f: Fn(UInt) -> (), v: UInt): () = f(v)
";
    check(input, OptLevel::O0);
}

#[test]
fn arrays() {
    let input = "
    fn main(): () = {
        let foo = [1, 2, 3]
        print fst(foo)
    }
    fn fst(a: [Int]): Int = 2
";
    check(input, OptLevel::O0);
}
