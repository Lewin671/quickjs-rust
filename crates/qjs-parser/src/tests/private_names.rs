use qjs_ast::{ClassBody, ClassElement, ClassMemberKey, Expr, MemberProperty, MethodKind, Stmt};

use crate::parse_script;

fn class_body(source: &str) -> ClassBody {
    let script = parse_script(source).expect("source should parse");
    let [Stmt::ClassDecl { body, .. }] = script.body.as_slice() else {
        panic!("expected a single class declaration");
    };
    body.clone()
}

#[test]
fn parses_private_instance_and_static_fields() {
    let body = class_body("class C { #x = 1; #y; static #s = 2; }");
    let keys: Vec<_> = body
        .elements
        .iter()
        .map(|element| match element {
            ClassElement::Field(field) => (field.key.clone(), field.is_static),
            ClassElement::Method(_) | ClassElement::StaticBlock(_) => {
                panic!("expected only fields")
            }
        })
        .collect();
    assert_eq!(
        keys,
        vec![
            (ClassMemberKey::Private("x".to_owned()), false),
            (ClassMemberKey::Private("y".to_owned()), false),
            (ClassMemberKey::Private("s".to_owned()), true),
        ]
    );
}

#[test]
fn parses_private_methods_and_accessors() {
    let body = class_body(
        "class C { #m() {} static #sm() {} get #g() {} set #g(v) {} #ref() { return this.#m(); } }",
    );
    let kinds: Vec<_> = body
        .elements
        .iter()
        .filter_map(|element| match element {
            ClassElement::Method(member) => {
                Some((member.key.clone(), member.kind, member.is_static))
            }
            ClassElement::Field(_) | ClassElement::StaticBlock(_) => None,
        })
        .collect();
    assert_eq!(
        kinds,
        vec![
            (
                ClassMemberKey::Private("m".to_owned()),
                MethodKind::Method,
                false
            ),
            (
                ClassMemberKey::Private("sm".to_owned()),
                MethodKind::Method,
                true
            ),
            (
                ClassMemberKey::Private("g".to_owned()),
                MethodKind::Getter,
                false
            ),
            (
                ClassMemberKey::Private("g".to_owned()),
                MethodKind::Setter,
                false
            ),
            (
                ClassMemberKey::Private("ref".to_owned()),
                MethodKind::Method,
                false
            ),
        ]
    );
}

#[test]
fn parses_private_member_access() {
    let body = class_body("class C { #x; m() { return this.#x; } }");
    let ClassElement::Method(member) = &body.elements[1] else {
        panic!("expected a method");
    };
    let Expr::Function { body, .. } = &member.value else {
        panic!("method value should be a function");
    };
    let Stmt::Return {
        argument: Some(Expr::Member { property, .. }),
        ..
    } = &body[0]
    else {
        panic!("expected a private member return");
    };
    assert_eq!(*property, MemberProperty::Private("x".to_owned()));
}

#[test]
fn parses_private_brand_check() {
    let body = class_body("class C { #x; has(o) { return #x in o; } }");
    let ClassElement::Method(member) = &body.elements[1] else {
        panic!("expected a method");
    };
    let Expr::Function { body, .. } = &member.value else {
        panic!("method value should be a function");
    };
    let Stmt::Return {
        argument: Some(Expr::PrivateIn { name, .. }),
        ..
    } = &body[0]
    else {
        panic!("expected a `#x in o` brand check");
    };
    assert_eq!(name, "x");
}

#[test]
fn allows_getter_setter_private_pair() {
    parse_script("class C { get #g() {} set #g(v) {} }").expect("a get/set pair is allowed");
    parse_script("class C { static get #g() {} static set #g(v) {} }")
        .expect("a static get/set pair is allowed");
}

#[test]
fn rejects_duplicate_private_names() {
    parse_script("class C { #x; #x; }").expect_err("duplicate private field is an error");
    parse_script("class C { #m() {} #m() {} }").expect_err("duplicate private method is an error");
    parse_script("class C { #x; #x() {} }")
        .expect_err("private field and method with the same name clash");
    parse_script("class C { get #g() {} get #g() {} }")
        .expect_err("two getters for the same name clash");
    parse_script("class C { get #g() {} static set #g(v) {} }")
        .expect_err("a get/set pair must share static-ness");
}

#[test]
fn rejects_private_constructor() {
    parse_script("class C { #constructor() {} }").expect_err("#constructor is a syntax error");
    parse_script("class C { #constructor; }").expect_err("#constructor field is a syntax error");
}

#[test]
fn rejects_undeclared_private_name() {
    parse_script("class C { m() { return this.#missing; } }")
        .expect_err("undeclared private name is a syntax error");
    parse_script("class C { has(o) { return #missing in o; } }")
        .expect_err("undeclared private name in a brand check is a syntax error");
}

#[test]
fn forward_reference_within_class_resolves() {
    parse_script("class C { use() { return this.#later; } #later = 1; }")
        .expect("a forward reference to a later-declared private name is legal");
}

#[test]
fn nested_class_sees_outer_private_name() {
    parse_script(
        "class Outer { #o; make() { return class Inner { read(self) { return self.#o; } }; } }",
    )
    .expect("an inner class may reference an outer private name");
}

#[test]
fn rejects_delete_of_private_member() {
    parse_script("class C { #x; m() { delete this.#x; } }")
        .expect_err("deleting a private member is a syntax error");
}

#[test]
fn rejects_private_name_outside_class() {
    parse_script("this.#x;").expect_err("a private member access outside a class is an error");
    parse_script("#x;").expect_err("a bare private name outside a class is an error");
}
