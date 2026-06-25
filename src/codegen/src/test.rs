use errors::ErrorHandler;

use crate::{CodegenMode, OptLevel, Target};

fn check(src: &str, opt_level: OptLevel) {
    let mut hir = nameres::test_resolve_ast(src).unwrap();
    let expr_tys = typecheck::type_hir(&mut hir, ErrorHandler::TEST).unwrap();
    let mir = lower::lower(ErrorHandler::TEST, &hir, &expr_tys);
    crate::emit(
        &mir,
        opt_level,
        CodegenMode::Silent,
        Target::default(),
        "test",
    );
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
        let mut m = 3
        apply(fn(n) -> print n + m, 2)
    }
    fn apply(f: Fn(UInt) -> (), v: UInt): () = f(v)
";
    check(input, OptLevel::O0);
}

#[test]
fn closure_capture() {
    let input = "
    fn main(): () = {
        let closure = make_closure()
        closure(1)
    }

    fn make_closure(): Fn(UInt) -> () = {
        let array: [Int] = [1, 2, 3, 4, 5]
        fn(index) -> print array.[index]
    }
";
    check(input, OptLevel::O0)
}

#[test]
fn func_ptr() {
    let input = "
    fn main(): () = {
        let f = print_
        apply_2(f)
    }
    fn print_(n: Int): () = print n
    fn apply_2(f: Fn(Int) -> ()): () = f(2)
";
    check(input, OptLevel::O0);
}

#[test]
fn arrays() {
    let input = "
    fn main(): () = {
        let foo = [1, 2, 3]
        print fst(foo)
        print foo == []
    }
    fn fst(a: [Int]): Int = a.[0]
";
    check(input, OptLevel::O0);
}

#[test]
fn unit_param() {
    let input = "
    fn main(): () = {
        let mut v = ()
        stupid((), mut v, 42)
    }
    fn stupid(x: (), mut y: (), z: Int): () = {
        y = x
        y
    }
";
    check(input, OptLevel::O0);
}

#[test]
fn bad_call() {
    check("fn foo(): Int = [1, 2, 3].[0]", OptLevel::O0);
}

#[test]
fn not_array() {
    let input = "
    fn main(): () = {
        let mut a = [1]
        let b = a
        a.[0] = 2
        print b.[0]
    }
";
    check(input, OptLevel::O0);
}
