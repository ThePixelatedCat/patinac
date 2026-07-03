use std::assert_matches;

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

    let input = "
    record Foo()
    const Foo: Foo = Foo()
";
    assert_matches!(crate::test_resolve_ast(input), Ok(_));
}

#[test]
fn so_much_int() {
    let input = "
    record Int()
    const Int: Int = Int()
";
    assert_matches!(crate::test_resolve_ast(input), Ok(_))
}

#[test]
fn assoc_items() {
    let input = "
    import MyInt::ZERO

    record MyInt(val: Int)
    impl MyInt {
        const ZERO: MyInt = MyInt(0)
        fn add(lhs: MyInt, rhs: MyInt): MyInt = MyInt(lhs.val + rhs.val)
    }

    fn bad_identity(int: MyInt): MyInt = MyInt::add(int, ZERO)
";
    assert_matches!(crate::test_resolve_ast(input), Ok(_))
}
