use qjs_ast::{ClassMemberKey, Expr, MethodKind, Stmt};

use crate::parse_script;

#[test]
fn parses_class_declaration_with_constructor_and_methods() {
    let script =
        parse_script("class Point { constructor(x, y) { this.x = x; } norm() { return 0; } }")
            .expect("source should parse");
    let [Stmt::ClassDecl { name, body, span }] = script.body.as_slice() else {
        panic!("expected class declaration");
    };
    assert_eq!(name, "Point");
    assert_eq!(span.start, 0);
    assert_eq!(body.members.len(), 2);

    let constructor = &body.members[0];
    assert_eq!(constructor.kind, MethodKind::Constructor);
    assert_eq!(
        constructor.key,
        ClassMemberKey::Literal("constructor".to_owned())
    );
    let Expr::Function {
        name: Some(ctor_name),
        params,
        constructable,
        ..
    } = &constructor.value
    else {
        panic!("constructor value should be a function expression");
    };
    assert_eq!(ctor_name, "constructor");
    assert_eq!(params.names(), ["x", "y"]);
    assert!(!constructable, "class methods are not constructable");

    let method = &body.members[1];
    assert_eq!(method.kind, MethodKind::Method);
    assert_eq!(method.key, ClassMemberKey::Literal("norm".to_owned()));
}

#[test]
fn parses_class_expression_named_and_anonymous() {
    let script = parse_script("let c = class Named { m() {} };").expect("source should parse");
    let [Stmt::VarDecl { declarations, .. }] = script.body.as_slice() else {
        panic!("expected variable declaration");
    };
    let Some(Expr::Class {
        name: Some(name),
        body,
        ..
    }) = &declarations[0].init
    else {
        panic!("expected named class expression");
    };
    assert_eq!(name, "Named");
    assert_eq!(body.members.len(), 1);

    let script = parse_script("let c = class { m() {} };").expect("source should parse");
    let [Stmt::VarDecl { declarations, .. }] = script.body.as_slice() else {
        panic!("expected variable declaration");
    };
    let Some(Expr::Class { name: None, .. }) = &declarations[0].init else {
        panic!("expected anonymous class expression");
    };
}

#[test]
fn parses_empty_class_body() {
    let script = parse_script("class Empty {}").expect("source should parse");
    let [Stmt::ClassDecl { body, .. }] = script.body.as_slice() else {
        panic!("expected class declaration");
    };
    assert!(body.members.is_empty());
}

#[test]
fn allows_semicolons_between_members() {
    let script = parse_script("class C { ; a() {}; ; b() {} ; }").expect("source should parse");
    let [Stmt::ClassDecl { body, .. }] = script.body.as_slice() else {
        panic!("expected class declaration");
    };
    assert_eq!(body.members.len(), 2);
    assert_eq!(body.members[0].key, ClassMemberKey::Literal("a".to_owned()));
    assert_eq!(body.members[1].key, ClassMemberKey::Literal("b".to_owned()));
}

#[test]
fn allows_keyword_named_methods() {
    let script = parse_script("class C { if() {} return() {} }").expect("source should parse");
    let [Stmt::ClassDecl { body, .. }] = script.body.as_slice() else {
        panic!("expected class declaration");
    };
    assert_eq!(body.members.len(), 2);
    assert_eq!(
        body.members[0].key,
        ClassMemberKey::Literal("if".to_owned())
    );
    assert_eq!(
        body.members[1].key,
        ClassMemberKey::Literal("return".to_owned())
    );
}

#[test]
fn rejects_duplicate_constructor() {
    let error = parse_script("class C { constructor() {} constructor() {} }")
        .expect_err("two constructors should be rejected");
    assert!(error.message.contains("constructor"));
}

#[test]
fn parses_static_methods_and_accessors() {
    let script = parse_script("class C { static m() {} static get x() {} static set x(v) {} }")
        .expect("source should parse");
    let [Stmt::ClassDecl { body, .. }] = script.body.as_slice() else {
        panic!("expected class declaration");
    };
    assert_eq!(body.members.len(), 3);
    assert!(body.members[0].is_static);
    assert_eq!(body.members[0].kind, MethodKind::Method);
    assert!(body.members[1].is_static);
    assert_eq!(body.members[1].kind, MethodKind::Getter);
    assert!(body.members[2].is_static);
    assert_eq!(body.members[2].kind, MethodKind::Setter);
}

#[test]
fn parses_instance_accessors() {
    let script = parse_script("class C { get x() {} set x(v) {} }").expect("source should parse");
    let [Stmt::ClassDecl { body, .. }] = script.body.as_slice() else {
        panic!("expected class declaration");
    };
    assert_eq!(body.members.len(), 2);
    assert_eq!(body.members[0].kind, MethodKind::Getter);
    assert!(!body.members[0].is_static);
    assert_eq!(body.members[0].key, ClassMemberKey::Literal("x".to_owned()));
    assert_eq!(body.members[1].kind, MethodKind::Setter);
}

#[test]
fn parses_computed_member_names() {
    let script = parse_script("class C { [a]() {} static [b]() {} }")
        .expect("computed member names should parse");
    let [Stmt::ClassDecl { body, .. }] = script.body.as_slice() else {
        panic!("expected class declaration");
    };
    assert_eq!(body.members.len(), 2);
    assert!(matches!(body.members[0].key, ClassMemberKey::Computed(_)));
    assert!(!body.members[0].is_static);
    assert!(matches!(body.members[1].key, ClassMemberKey::Computed(_)));
    assert!(body.members[1].is_static);
}

#[test]
fn allows_static_get_set_as_plain_method_names() {
    let script = parse_script("class C { static() {} get() {} set() {} }")
        .expect("static/get/set are valid method names");
    let [Stmt::ClassDecl { body, .. }] = script.body.as_slice() else {
        panic!("expected class declaration");
    };
    assert_eq!(body.members.len(), 3);
    for member in &body.members {
        assert_eq!(member.kind, MethodKind::Method);
        assert!(!member.is_static);
    }
    assert_eq!(
        body.members[0].key,
        ClassMemberKey::Literal("static".to_owned())
    );
    assert_eq!(
        body.members[1].key,
        ClassMemberKey::Literal("get".to_owned())
    );
    assert_eq!(
        body.members[2].key,
        ClassMemberKey::Literal("set".to_owned())
    );
}

#[test]
fn parses_string_and_numeric_method_names() {
    let script = parse_script("class C { \"str\"() {} 1() {} }").expect("source should parse");
    let [Stmt::ClassDecl { body, .. }] = script.body.as_slice() else {
        panic!("expected class declaration");
    };
    assert_eq!(body.members.len(), 2);
    assert_eq!(
        body.members[0].key,
        ClassMemberKey::Literal("str".to_owned())
    );
    assert_eq!(body.members[1].key, ClassMemberKey::Literal("1".to_owned()));
}

#[test]
fn rejects_getter_with_parameters() {
    let error = parse_script("class C { get x(a) {} }").expect_err("getters take no parameters");
    assert!(error.message.contains("getter"));
}

#[test]
fn rejects_setter_with_wrong_parameter_count() {
    assert!(parse_script("class C { set x() {} }").is_err());
    assert!(parse_script("class C { set x(a, b) {} }").is_err());
    assert!(parse_script("class C { set x(...rest) {} }").is_err());
}

#[test]
fn rejects_accessor_named_constructor() {
    let error = parse_script("class C { get constructor() {} }")
        .expect_err("constructor may not be an accessor");
    assert!(error.message.contains("constructor"));
}

#[test]
fn allows_static_accessor_named_constructor() {
    parse_script("class C { static get constructor() {} }")
        .expect("static constructor accessor is allowed");
}

#[test]
fn rejects_static_prototype() {
    let error = parse_script("class C { static prototype() {} }")
        .expect_err("static prototype is forbidden");
    assert!(error.message.contains("prototype"));
}

#[test]
fn rejects_generator_methods() {
    let error = parse_script("class C { *gen() {} }").expect_err("generators are out of scope");
    assert!(error.message.contains("generator"));
}

#[test]
fn rejects_extends_clause() {
    let error = parse_script("class C extends D {}").expect_err("extends is out of scope");
    assert!(error.message.contains("extends"));
}

#[test]
fn rejects_class_fields() {
    let error = parse_script("class C { x = 1; }").expect_err("class fields are out of scope");
    assert!(error.message.contains("field"));
}

#[test]
fn does_not_panic_on_malformed_class() {
    assert!(parse_script("class").is_err());
    assert!(parse_script("class {").is_err());
    assert!(parse_script("class C {").is_err());
    assert!(parse_script("class C { m( }").is_err());
    assert!(parse_script("class 123 {}").is_err());
}
