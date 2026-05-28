use qjs_ast::{AssignmentTarget, Expr, MemberProperty, ObjectPropertyKey, Stmt};

use crate::parse_script;

#[test]
fn parses_function_declaration_and_call() {
    let script = parse_script("function add(a, b) { return a + b; } add(1, 2);").expect("source");
    let [
        Stmt::FunctionDecl { name, params, .. },
        Stmt::Expr(Expr::Call { arguments, .. }),
    ] = script.body.as_slice()
    else {
        panic!("expected function declaration followed by call");
    };
    assert_eq!(name, "add");
    assert_eq!(params, &["a", "b"]);
    assert_eq!(arguments.len(), 2);

    let script = parse_script("let f = function named(value) { return value; }; f(1);")
        .expect("source should parse");
    let [
        Stmt::VarDecl { declarations, .. },
        Stmt::Expr(Expr::Call { .. }),
    ] = script.body.as_slice()
    else {
        panic!("expected function expression assignment followed by call");
    };
    let Some(Expr::Function { name, params, .. }) = &declarations[0].init else {
        panic!("expected function expression initializer");
    };
    assert_eq!(name.as_deref(), Some("named"));
    assert_eq!(params, &["value"]);
}

#[test]
fn parses_new_expression() {
    let script = parse_script("new Point(1, 2);").expect("source should parse");
    let [
        Stmt::Expr(Expr::New {
            callee, arguments, ..
        }),
    ] = script.body.as_slice()
    else {
        panic!("expected one new expression statement");
    };
    assert!(matches!(callee.as_ref(), Expr::Identifier { name, .. } if name == "Point"));
    assert_eq!(arguments.len(), 2);
}

#[test]
fn parses_array_literal() {
    let script = parse_script("[1, 2 + 3,];").expect("source should parse");
    let [Stmt::Expr(Expr::Array { elements, .. })] = script.body.as_slice() else {
        panic!("expected one array expression");
    };
    assert_eq!(elements.len(), 2);
}

#[test]
fn parses_object_literal_and_member_assignment() {
    let script = parse_script("let object = { answer: 42, 'name': 7, }; object.answer = 43;")
        .expect("source should parse");
    let [
        Stmt::VarDecl { declarations, .. },
        Stmt::Expr(Expr::Assignment { target, .. }),
    ] = script.body.as_slice()
    else {
        panic!("expected object declaration followed by member assignment");
    };
    let Some(Expr::Object { properties, .. }) = &declarations[0].init else {
        panic!("expected object initializer");
    };
    assert_eq!(properties.len(), 2);
    assert_eq!(
        properties[0].key,
        ObjectPropertyKey::Literal("answer".to_owned())
    );
    assert_eq!(
        properties[1].key,
        ObjectPropertyKey::Literal("name".to_owned())
    );
    assert!(matches!(target, AssignmentTarget::Member { .. }));

    let script = parse_script("({ true: 1, false: 2, null: 3 });").expect("source should parse");
    let [Stmt::Expr(Expr::Object { properties, .. })] = script.body.as_slice() else {
        panic!("expected one object expression");
    };
    assert_eq!(
        properties[0].key,
        ObjectPropertyKey::Literal("true".to_owned())
    );
    assert_eq!(
        properties[1].key,
        ObjectPropertyKey::Literal("false".to_owned())
    );
    assert_eq!(
        properties[2].key,
        ObjectPropertyKey::Literal("null".to_owned())
    );

    let script =
        parse_script("let answer = 42; ({ answer, named: answer });").expect("source should parse");
    let [_, Stmt::Expr(Expr::Object { properties, .. })] = script.body.as_slice() else {
        panic!("expected object expression with shorthand property");
    };
    assert_eq!(properties.len(), 2);
    assert_eq!(
        properties[0].key,
        ObjectPropertyKey::Literal("answer".to_owned())
    );
    assert!(matches!(
        properties[0].value,
        Expr::Identifier { ref name, .. } if name == "answer"
    ));

    let script = parse_script("let key = 'answer'; ({ [key]: 42 });").expect("source should parse");
    let [_, Stmt::Expr(Expr::Object { properties, .. })] = script.body.as_slice() else {
        panic!("expected object expression with computed property");
    };
    assert!(matches!(
        properties[0].key,
        ObjectPropertyKey::Computed(Expr::Identifier { ref name, .. }) if name == "key"
    ));

    let script =
        parse_script("({ method(a, b) { return a + b; } });").expect("source should parse");
    let [Stmt::Expr(Expr::Object { properties, .. })] = script.body.as_slice() else {
        panic!("expected object expression with method definition");
    };
    assert_eq!(
        properties[0].key,
        ObjectPropertyKey::Literal("method".to_owned())
    );
    let Expr::Function {
        name,
        params,
        constructable,
        ..
    } = &properties[0].value
    else {
        panic!("expected method value to parse as function expression");
    };
    assert_eq!(name.as_deref(), Some("method"));
    assert_eq!(params, &["a".to_owned(), "b".to_owned()]);
    assert!(!constructable);
}

#[test]
fn parses_member_access() {
    let script = parse_script("items[0].length;").expect("source should parse");
    let [
        Stmt::Expr(Expr::Member {
            object, property, ..
        }),
    ] = script.body.as_slice()
    else {
        panic!("expected member expression");
    };
    assert_eq!(property, &MemberProperty::Named("length".to_owned()));
    assert!(matches!(object.as_ref(), Expr::Member { .. }));
}
