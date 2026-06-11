use std::assert_matches;

use parse::Parser;

#[test]
fn lambda() {
    let input = "{
    let mut a = 5
    let b = 6
    fn(c) -> a + b + c
    a = 6
}";
    assert_matches!(crate::test_resolve_expr(input), Ok(_));
}

#[test]
fn for_() {
    assert_matches!(
        crate::test_resolve_expr("for x in [1, 2, 3] { x + 5 }"),
        Ok(_)
    );
}

#[test]
fn shadowing() {
    let input = r#"{
    let a: UInt = 5
    let a = "Hello, World"
    {let a = true}
    a
}"#;
    assert_matches!(crate::test_resolve_expr(input), Ok(_));
}

#[test]
fn fib() {
    let input = "
    fn fib(n: UInt): UInt =
        if n <= 1 { n } else { fib(n - 1) + fib(n - 2) }         
";
    assert_matches!(crate::test_resolve_ast(input), Ok(_));
}

#[test]
fn unbound_var() {
    assert_matches!(crate::test_resolve_expr("a + 5"), Err(_));
}

#[test]
fn unique_places() {
    let input = "
    fn main(): () = {
        let f = fn(mut a, b) -> a = a + b
        let b = 5
        let g = fn(mut a) -> f(mut a, b)
    }
";
    assert_matches!(crate::test_resolve_ast(input), Ok(_));

    let input = "
    record Box(v: Int)
    fn main(): () = {
        let f = fn(mut a, b) -> a = a + b.v
        let g = fn(mut a) -> f(mut a.v, a)
    }
";
    assert_matches!(crate::test_resolve_ast(input), Err(_));

    let input = "
    fn main(): () = {
        let mut a = 1
        swap(mut a, mut a)
    }
    fn swap(mut a: Int, mut b: Int): () = {}
";
    assert_matches!(crate::test_resolve_ast(input), Err(_));

    let input = "
    fn main(): () = {
        let mut foo = [1, 1]
        swap(mut foo.[0], mut foo.[0])
    }
    fn swap(mut a: Int, mut b: Int): () = {}
";
    assert_matches!(crate::test_resolve_ast(input), Err(_));

    let input = "
    fn main(): () = {
        let mut foo = [1, 1]
        swap(mut foo.[0], mut foo.[1])
    }
    fn swap(mut a: Int, mut b: Int): () = {}
";
    assert_matches!(crate::test_resolve_ast(input), Ok(_));
}

#[test]
fn consts() {
    let input = "
    const B: UInt = A * 2
    const A: UInt = 5
";
    assert_matches!(crate::test_resolve_ast(input), Ok(_));
}

#[test]
fn list() {
    let input = r#"
    record List(head: Link)

    record Link(elem: Int, next: Link)

    fn cons(list: List, elem: Int): List = "todo"
"#;
    assert_matches!(crate::test_resolve_ast(input), Ok(_));
}

#[test]
fn unknown_type() {
    assert_matches!(
        crate::test_resolve_expr(r#"{let x: Foo = "Hello, World!"}"#),
        Err(_)
    );
}

#[test]
fn rebinding() {
    let input = "
    const A: UInt = 5
    const A: UInt = 6
";
    assert_matches!(crate::test_resolve_ast(input), Err(_));

    let input = "
    record Foo()
    record Foo(val: UInt)
";
    assert_matches!(crate::test_resolve_ast(input), Err(_));
}

#[test]
#[allow(clippy::unwrap_used, reason = "test function")]
fn modules() {
    let root_ast = Parser::new_test("fn main(): () = print foo::sum(1, 1)")
        .parse()
        .unwrap();
    let foo_ast = Parser::new_test("fn sum(a: Int, b: Int): Int = a + b")
        .parse()
        .unwrap();

    todo!()
    // let tree = ModuleTree {
    //     name: String::from("main"),
    //     contents: root_ast,
    //     children: vec![ModuleTree {
    //         name: String::from("foo"),
    //         contents: foo_ast,
    //         children: vec![],
    //     }],
    // };
    // assert_matches!(crate::resolve(tree.into(), ErrorHandler::TEST), Err(_));
}
