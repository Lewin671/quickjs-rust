use qjs_ast::{
    ClassBody, ClassElement, ClassField, ClassMember, ClassMemberKey, Expr, MethodKind, Stmt,
};

use crate::{parse_module, parse_script};

/// Collects the method/accessor/constructor members of a class body, ignoring
/// fields.
fn members(body: &ClassBody) -> Vec<&ClassMember> {
    body.elements
        .iter()
        .filter_map(|element| match element {
            ClassElement::Method(member) => Some(member),
            ClassElement::Field(_) | ClassElement::StaticBlock(_) => None,
        })
        .collect()
}

/// Collects the field elements of a class body.
fn fields(body: &ClassBody) -> Vec<&ClassField> {
    body.elements
        .iter()
        .filter_map(|element| match element {
            ClassElement::Field(field) => Some(field),
            ClassElement::Method(_) | ClassElement::StaticBlock(_) => None,
        })
        .collect()
}

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
    assert_eq!(members(body).len(), 2);

    let constructor = members(body)[0];
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

    let method = members(body)[1];
    assert_eq!(method.kind, MethodKind::Method);
    assert_eq!(method.key, ClassMemberKey::Literal("norm".to_owned()));
}

#[test]
fn parses_static_initialization_blocks() {
    let script = parse_script("class C { static x = 1; static { this.y = 2; } }")
        .expect("static blocks should parse");
    let [Stmt::ClassDecl { body, .. }] = script.body.as_slice() else {
        panic!("expected class declaration");
    };
    // The block is a distinct element, in source order after the field.
    assert!(matches!(body.elements[0], ClassElement::Field(_)));
    let ClassElement::StaticBlock(block) = &body.elements[1] else {
        panic!("expected a static initialization block");
    };
    assert_eq!(block.body.len(), 1);
}

#[test]
fn static_is_a_method_name_when_not_followed_by_a_block() {
    // `static() {}` and `static = 1` use `static` as the member name, not as the
    // block/modifier keyword.
    let script = parse_script("class C { static() {} }").expect("source should parse");
    let [Stmt::ClassDecl { body, .. }] = script.body.as_slice() else {
        panic!("expected class declaration");
    };
    assert_eq!(
        members(body)[0].key,
        ClassMemberKey::Literal("static".to_owned())
    );
    assert!(!members(body)[0].is_static);
}

#[test]
fn canonicalizes_numeric_member_keys() {
    let script =
        parse_script("class C { 0b10() {} get 0x10() {} 1.0() {} }").expect("source should parse");
    let [Stmt::ClassDecl { body, .. }] = script.body.as_slice() else {
        panic!("expected class declaration");
    };
    let keys: Vec<&ClassMemberKey> = members(body).iter().map(|member| &member.key).collect();
    assert_eq!(keys[0], &ClassMemberKey::Literal("2".to_owned()));
    assert_eq!(keys[1], &ClassMemberKey::Literal("16".to_owned()));
    assert_eq!(keys[2], &ClassMemberKey::Literal("1".to_owned()));
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
    assert_eq!(members(body).len(), 1);

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
    assert!(members(body).is_empty());
}

#[test]
fn allows_semicolons_between_members() {
    let script = parse_script("class C { ; a() {}; ; b() {} ; }").expect("source should parse");
    let [Stmt::ClassDecl { body, .. }] = script.body.as_slice() else {
        panic!("expected class declaration");
    };
    assert_eq!(members(body).len(), 2);
    assert_eq!(
        members(body)[0].key,
        ClassMemberKey::Literal("a".to_owned())
    );
    assert_eq!(
        members(body)[1].key,
        ClassMemberKey::Literal("b".to_owned())
    );
}

#[test]
fn allows_keyword_named_methods() {
    let script = parse_script("class C { if() {} return() {} }").expect("source should parse");
    let [Stmt::ClassDecl { body, .. }] = script.body.as_slice() else {
        panic!("expected class declaration");
    };
    assert_eq!(members(body).len(), 2);
    assert_eq!(
        members(body)[0].key,
        ClassMemberKey::Literal("if".to_owned())
    );
    assert_eq!(
        members(body)[1].key,
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
    assert_eq!(members(body).len(), 3);
    assert!(members(body)[0].is_static);
    assert_eq!(members(body)[0].kind, MethodKind::Method);
    assert!(members(body)[1].is_static);
    assert_eq!(members(body)[1].kind, MethodKind::Getter);
    assert!(members(body)[2].is_static);
    assert_eq!(members(body)[2].kind, MethodKind::Setter);
}

#[test]
fn parses_instance_accessors() {
    let script = parse_script("class C { get x() {} set x(v) {} }").expect("source should parse");
    let [Stmt::ClassDecl { body, .. }] = script.body.as_slice() else {
        panic!("expected class declaration");
    };
    assert_eq!(members(body).len(), 2);
    assert_eq!(members(body)[0].kind, MethodKind::Getter);
    assert!(!members(body)[0].is_static);
    assert_eq!(
        members(body)[0].key,
        ClassMemberKey::Literal("x".to_owned())
    );
    assert_eq!(members(body)[1].kind, MethodKind::Setter);
}

#[test]
fn parses_computed_member_names() {
    let script = parse_script("class C { [a]() {} static [b]() {} }")
        .expect("computed member names should parse");
    let [Stmt::ClassDecl { body, .. }] = script.body.as_slice() else {
        panic!("expected class declaration");
    };
    assert_eq!(members(body).len(), 2);
    assert!(matches!(members(body)[0].key, ClassMemberKey::Computed(_)));
    assert!(!members(body)[0].is_static);
    assert!(matches!(members(body)[1].key, ClassMemberKey::Computed(_)));
    assert!(members(body)[1].is_static);
}

#[test]
fn allows_static_get_set_as_plain_method_names() {
    let script = parse_script("class C { static() {} get() {} set() {} }")
        .expect("static/get/set are valid method names");
    let [Stmt::ClassDecl { body, .. }] = script.body.as_slice() else {
        panic!("expected class declaration");
    };
    assert_eq!(members(body).len(), 3);
    for member in members(body) {
        assert_eq!(member.kind, MethodKind::Method);
        assert!(!member.is_static);
    }
    assert_eq!(
        members(body)[0].key,
        ClassMemberKey::Literal("static".to_owned())
    );
    assert_eq!(
        members(body)[1].key,
        ClassMemberKey::Literal("get".to_owned())
    );
    assert_eq!(
        members(body)[2].key,
        ClassMemberKey::Literal("set".to_owned())
    );
}

#[test]
fn parses_string_and_numeric_method_names() {
    let script = parse_script("class C { \"str\"() {} 1() {} }").expect("source should parse");
    let [Stmt::ClassDecl { body, .. }] = script.body.as_slice() else {
        panic!("expected class declaration");
    };
    assert_eq!(members(body).len(), 2);
    assert_eq!(
        members(body)[0].key,
        ClassMemberKey::Literal("str".to_owned())
    );
    assert_eq!(
        members(body)[1].key,
        ClassMemberKey::Literal("1".to_owned())
    );
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
fn parses_class_generator_method() {
    let script = parse_script("class C { *gen() {} }").expect("generator methods parse");
    let [Stmt::ClassDecl { body, .. }] = script.body.as_slice() else {
        panic!("expected class declaration");
    };
    let [ClassElement::Method(member)] = body.elements.as_slice() else {
        panic!("expected one method element");
    };
    let Expr::Function { is_generator, .. } = &member.value else {
        panic!("expected method function value");
    };
    assert!(*is_generator);
}

#[test]
fn parses_extends_clause_with_identifier_heritage() {
    let script = parse_script("class C extends D {}").expect("source should parse");
    let [Stmt::ClassDecl { body, .. }] = script.body.as_slice() else {
        panic!("expected class declaration");
    };
    let Some(heritage) = &body.heritage else {
        panic!("expected heritage");
    };
    let Expr::Identifier { name, .. } = heritage.as_ref() else {
        panic!("expected identifier heritage");
    };
    assert_eq!(name, "D");
}

#[test]
fn parses_extends_clause_with_member_and_call_heritage() {
    let script = parse_script("class C extends mixins.Base(Object) {}").expect("source parses");
    let [Stmt::ClassDecl { body, .. }] = script.body.as_slice() else {
        panic!("expected class declaration");
    };
    assert!(matches!(body.heritage.as_deref(), Some(Expr::Call { .. })));

    let script = parse_script("let c = class extends null {};").expect("source parses");
    let [Stmt::VarDecl { declarations, .. }] = script.body.as_slice() else {
        panic!("expected variable declaration");
    };
    let Some(Expr::Class { body, .. }) = &declarations[0].init else {
        panic!("expected class expression");
    };
    assert!(matches!(body.heritage.as_deref(), Some(Expr::Literal(_))));
}

#[test]
fn parses_super_member_and_call_in_methods() {
    parse_script("class C extends D { constructor() { super(); super.x; super.m(); } }")
        .expect("super inside derived constructor parses");
    parse_script("class C extends D { m() { return super.n() + super['k']; } }")
        .expect("super property access inside a method parses");
    parse_script("class C extends D { static s() { return super.t; } }")
        .expect("super property access inside a static method parses");
}

#[test]
fn rejects_super_outside_valid_contexts() {
    assert!(parse_script("super.x;").is_err());
    assert!(parse_script("super();").is_err());
    assert!(parse_script("function f() { super.x; }").is_err());
    assert!(parse_script("function f() { super(); }").is_err());
    // `super()` is not allowed in a non-derived constructor or a plain method.
    assert!(parse_script("class C { constructor() { super(); } }").is_err());
    assert!(parse_script("class C extends D { m() { super(); } }").is_err());
    // A bare `super` reference is always a syntax error.
    assert!(parse_script("class C extends D { m() { super; } }").is_err());
}

#[test]
fn parses_public_fields_with_and_without_initializers() {
    let script = parse_script("class C { x = 1; y; static z = 2; }").expect("fields should parse");
    let [Stmt::ClassDecl { body, .. }] = script.body.as_slice() else {
        panic!("expected class declaration");
    };
    let fields = fields(body);
    assert_eq!(fields.len(), 3);
    assert_eq!(fields[0].key, ClassMemberKey::Literal("x".to_owned()));
    assert!(fields[0].initializer.is_some());
    assert!(!fields[0].is_static);
    assert_eq!(fields[1].key, ClassMemberKey::Literal("y".to_owned()));
    assert!(fields[1].initializer.is_none());
    assert_eq!(fields[2].key, ClassMemberKey::Literal("z".to_owned()));
    assert!(fields[2].is_static);
}

#[test]
fn parses_computed_string_and_numeric_field_keys() {
    let script = parse_script("class C { [a] = 1; \"s\" = 2; 3 = 4; }").expect("field keys parse");
    let [Stmt::ClassDecl { body, .. }] = script.body.as_slice() else {
        panic!("expected class declaration");
    };
    let fields = fields(body);
    assert_eq!(fields.len(), 3);
    assert!(matches!(fields[0].key, ClassMemberKey::Computed(_)));
    assert_eq!(fields[1].key, ClassMemberKey::Literal("s".to_owned()));
    assert_eq!(fields[2].key, ClassMemberKey::Literal("3".to_owned()));
}

#[test]
fn field_asi_allows_newline_termination() {
    parse_script("class C {\n  x = 1\n  y = 2\n}").expect("newline terminates a field");
}

#[test]
fn field_asi_rejects_two_fields_on_one_line() {
    let error = parse_script("class C { x = 1 y = 2 }")
        .expect_err("two fields on one line without `;` is an error");
    assert!(error.message.contains("class field"));
}

#[test]
fn rejects_instance_field_named_constructor() {
    let error = parse_script("class C { constructor = 1; }")
        .expect_err("instance field `constructor` is a syntax error");
    assert!(error.message.contains("constructor"));
}

#[test]
fn rejects_static_field_named_prototype() {
    let error = parse_script("class C { static prototype = 1; }")
        .expect_err("static field `prototype` is a syntax error");
    assert!(error.message.contains("prototype"));
}

#[test]
fn allows_field_named_static_or_get() {
    let script = parse_script("class C { static = 1; get = 2; }").expect("fields parse");
    let [Stmt::ClassDecl { body, .. }] = script.body.as_slice() else {
        panic!("expected class declaration");
    };
    let fields = fields(body);
    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0].key, ClassMemberKey::Literal("static".to_owned()));
    assert!(!fields[0].is_static);
    assert_eq!(fields[1].key, ClassMemberKey::Literal("get".to_owned()));
}

#[test]
fn rejects_reserved_class_binding_names() {
    for source in [
        "class static {}",
        "class st\\u0061tic {}",
        "class l\\u0065t {}",
        "class yield {}",
        "(class static {})",
    ] {
        let error = parse_script(source).expect_err("reserved class binding name should fail");
        assert!(error.message.contains("class binding name"));
    }

    for source in ["class await {}", "class aw\\u0061it {}"] {
        let error = parse_module(source).expect_err("await class binding name should fail");
        assert!(error.message.contains("class binding name"));
    }
}

#[test]
fn rejects_static_block_early_error_contexts() {
    for source in [
        "class C { static { return; } }",
        "class C { static { arguments; } }",
        "class C { static { await; } }",
        "class C { static { yield; } }",
        "class C { static { class await {} } }",
        "class C { static { let await; } }",
    ] {
        parse_script(source).expect_err("static block early error should fail");
    }
}

#[test]
fn static_block_context_does_not_cross_function_boundaries() {
    parse_script("class C { static { function f() { return arguments; } } }")
        .expect("ordinary function should reset static block early-error context");

    parse_script(
        "class C { static { class Nested { method({x = arguments}) { return arguments; } } } }",
    )
    .expect("class method parameters and body should reset static block context");
}

#[test]
fn static_block_context_does_not_cross_arrow_body_boundary() {
    parse_script("class C { static { (() => { class await {} }); } }")
        .expect("arrow body should reset static block early-error context");
}

#[test]
fn allows_super_property_in_field_initializer() {
    parse_script("class C extends D { x = super.y; }")
        .expect("super.x is allowed in a field initializer");
}

#[test]
fn rejects_arguments_in_field_initializer() {
    let error = parse_script("class C { x = arguments; }")
        .expect_err("arguments is a syntax error in a field initializer");
    assert!(error.message.contains("arguments"));
}

#[test]
fn allows_arguments_in_nested_function_inside_field_initializer() {
    parse_script("class C { x = function () { return arguments; }; }")
        .expect("a nested function has its own arguments");
}

#[test]
fn does_not_panic_on_malformed_class() {
    assert!(parse_script("class").is_err());
    assert!(parse_script("class {").is_err());
    assert!(parse_script("class C {").is_err());
    assert!(parse_script("class C { m( }").is_err());
    assert!(parse_script("class 123 {}").is_err());
}
