use parse::Parser;
use typecheck::TypeChecker;

use crate::Codegen;

fn check(input: &str, opts: bool) {
    let toks = lex::lex(input).unwrap();
    let ast = Parser::new(toks).parse().unwrap();
    let mut hir = nameres::resolve(ast).unwrap();
    let ty_map = TypeChecker::default().type_program(&mut hir).unwrap();
    let ctx = crate::create_ctx();
    Codegen::new(&hir, &ty_map, &ctx, "test").codegen(opts);
}

#[test]
fn weird_sum() {
    let input = "
    fn sum(n: Int, m: Int): Int -> {
        let mut foo = n
        foo = foo + 1
        foo + {m - 1}
    }
";
    check(input, true);
}

#[test]
fn ifs() {
    let input = "
    fn foo(n: Float): Float -> 
        if n >= 0.0 { n } else { 0.0 } *. 2.0
";
    check(input, true);

    let input = "
    fn foo(n: Float): Float -> {
        let mut m = n *. 2.0
        if n < 0.0 { m = 0.0 }
        m
    }
";
    check(input, true);
}

#[test]
fn nested_if() {
    let input = "
    fn foo(a: Bool, b: Bool): Float -> {   
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
    check(input, true)
}

#[test]
fn call() {
    let input = "
    fn sum(n: Int, m: Int): Int -> n + m
    fn inc(n: Int): Int -> sum(n, 1)
";
    check(input, true)
}

#[test]
fn fib() {
    let input = "
    fn fib(n: Float): Float ->
        if n < 2.0 { n } else { fib(n -. 1.0) +. fib(n -. 2.0) }
";
    check(input, true);
}

#[test]
fn facs() {
    let input = "
    fn fac_rec(n: Float): Float -> 
        if n <= 0.0 { 1.0 } else { n *. fac_rec(n -. 1.0) }
";
    check(input, false);

    //     let input = "
    //     fn fac_iter(n: Float): Float -> {
    //         let mut f = 1
    //         let mut i = 1
    //         loop {
    //             if i > n { break }
    //             f = f * i
    //             i = i + 1
    //         }
    //         f
    //     }
    // ";
    //     check(input, true);
}

#[test]
fn mut_loop() {
    let input = "
    fn looping_inc(mut n: Int) -> 
        loop {
            n = n + 1
        }
";
    check(input, true);
}

#[test]
fn mut_arg() {
    let input = "
    fn inc(mut n: Int) -> n = n + 1
    fn do_inc() -> {
        let mut m = 5
        inc(mut m)
    }
";
    check(input, true)
}

#[test]
fn record_field() {
    let input = "
    record Point(x: Float, y: Float)
    fn get_x(self: Point): Float -> self.x
    fn get_y(self: Point): Float -> self.y
";
    check(input, true)
}
