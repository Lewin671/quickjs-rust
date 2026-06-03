use qjs_ast::{
    AssignmentTarget, Expr, Literal, MemberProperty, ObjectPropertyKey, ObjectPropertyKind, Stmt,
};

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

    let script = parse_script("let f = (a, b) => a + b;").expect("source should parse");
    let [Stmt::VarDecl { declarations, .. }] = script.body.as_slice() else {
        panic!("expected arrow function declaration");
    };
    let Some(Expr::Function {
        name,
        params,
        constructable,
        body,
        ..
    }) = &declarations[0].init
    else {
        panic!("expected arrow function initializer");
    };
    assert_eq!(name, &None);
    assert_eq!(params, &["a", "b"]);
    assert!(!constructable);
    assert!(matches!(body.as_slice(), [Stmt::Return { .. }]));

    let script = parse_script("let f = value => { return value; };").expect("source should parse");
    let [Stmt::VarDecl { declarations, .. }] = script.body.as_slice() else {
        panic!("expected single-parameter arrow function declaration");
    };
    let Some(Expr::Function { params, body, .. }) = &declarations[0].init else {
        panic!("expected arrow function initializer");
    };
    assert_eq!(params, &["value"]);
    assert!(matches!(body.as_slice(), [Stmt::Return { .. }]));
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
fn parses_member_access_after_new_expression() {
    let script = parse_script("new String('abc').length;").expect("source should parse");
    let [
        Stmt::Expr(Expr::Member {
            object, property, ..
        }),
    ] = script.body.as_slice()
    else {
        panic!("expected member access on new expression");
    };

    let Expr::New {
        callee, arguments, ..
    } = object.as_ref()
    else {
        panic!("expected member object to be a new expression");
    };
    assert!(matches!(callee.as_ref(), Expr::Identifier { name, .. } if name == "String"));
    assert_eq!(arguments.len(), 1);
    assert_eq!(property, &MemberProperty::Named("length".to_owned()));
}

#[test]
fn parses_array_literal() {
    let script = parse_script("[1, 2 + 3,];").expect("source should parse");
    let [Stmt::Expr(Expr::Array { elements, .. })] = script.body.as_slice() else {
        panic!("expected one array expression");
    };
    assert_eq!(elements.len(), 2);
    assert!(elements.iter().all(Option::is_some));
    let script = parse_script("[1, , 3];").expect("source should parse");
    let [Stmt::Expr(Expr::Array { elements, .. })] = script.body.as_slice() else {
        panic!("expected one array expression");
    };
    assert_eq!(elements.len(), 3);
    assert!(elements[1].is_none());
}

#[test]
fn parses_no_substitution_template_literal_as_string_literal() {
    let script = parse_script("`hello`;").expect("script should parse");
    let [Stmt::Expr(Expr::Literal(Literal::String { value, .. }))] = script.body.as_slice() else {
        panic!("expected one string literal expression");
    };
    assert_eq!(value, "hello");
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

    let script = parse_script("({ get value() { return 42; } });").expect("source should parse");
    let [Stmt::Expr(Expr::Object { properties, .. })] = script.body.as_slice() else {
        panic!("expected object expression with getter");
    };
    assert_eq!(
        properties[0].key,
        ObjectPropertyKey::Literal("value".to_owned())
    );
    assert_eq!(properties[0].kind, ObjectPropertyKind::Getter);
    let Expr::Function {
        name,
        params,
        constructable,
        ..
    } = &properties[0].value
    else {
        panic!("expected getter value to parse as function expression");
    };
    assert_eq!(name.as_deref(), Some("value"));
    assert!(params.is_empty());
    assert!(!constructable);

    let script = parse_script("({ get() { return 42; } });").expect("source should parse");
    let [Stmt::Expr(Expr::Object { properties, .. })] = script.body.as_slice() else {
        panic!("expected object expression with method named get");
    };
    assert_eq!(
        properties[0].key,
        ObjectPropertyKey::Literal("get".to_owned())
    );
    assert_eq!(properties[0].kind, ObjectPropertyKind::Data);
    let Expr::Function {
        name,
        params,
        constructable,
        ..
    } = &properties[0].value
    else {
        panic!("expected method value to parse as function expression");
    };
    assert_eq!(name.as_deref(), Some("get"));
    assert!(params.is_empty());
    assert!(!constructable);
}

#[test]
fn parses_regexp_literal_as_regexp_constructor_expression() {
    let script = parse_script("/./;").expect("source should parse");
    let [
        Stmt::Expr(Expr::New {
            callee, arguments, ..
        }),
    ] = script.body.as_slice()
    else {
        panic!("expected RegExp constructor expression");
    };

    assert!(matches!(
        callee.as_ref(),
        Expr::Identifier { name, .. } if name == "RegExp"
    ));
    assert!(matches!(
        arguments.as_slice(),
        [Expr::Literal(Literal::String { value, .. })] if value == "."
    ));
}

#[test]
fn parses_regexp_literal_with_escaped_atoms() {
    let script = parse_script(r#"/\(\)/;"#).expect("source should parse");
    let [
        Stmt::Expr(Expr::New {
            arguments, span, ..
        }),
    ] = script.body.as_slice()
    else {
        panic!("expected RegExp constructor expression");
    };

    assert_eq!(*span, qjs_ast::Span::new(0, 6));
    assert!(matches!(
        arguments.as_slice(),
        [Expr::Literal(Literal::String { value, span })]
            if value == r#"\(\)"# && *span == qjs_ast::Span::new(0, 6)
    ));
}

#[test]
fn parses_regexp_literal_with_character_class_range() {
    let script = parse_script(r#"/[0-9]/g;"#).expect("source should parse");
    let [
        Stmt::Expr(Expr::New {
            arguments, span, ..
        }),
    ] = script.body.as_slice()
    else {
        panic!("expected RegExp constructor expression");
    };

    assert_eq!(*span, qjs_ast::Span::new(0, 8));
    assert!(matches!(
        arguments.as_slice(),
        [
            Expr::Literal(Literal::String { value: pattern, span: pattern_span }),
            Expr::Literal(Literal::String { value: flags, span: flags_span })
        ] if pattern == "[0-9]"
            && *pattern_span == qjs_ast::Span::new(0, 7)
            && flags == "g"
            && *flags_span == qjs_ast::Span::new(7, 8)
    ));
}

#[test]
fn parses_regexp_literal_with_digit_escape_and_flags() {
    let script = parse_script(r#"/\d{2}/g;"#).expect("source should parse");
    let [
        Stmt::Expr(Expr::New {
            arguments, span, ..
        }),
    ] = script.body.as_slice()
    else {
        panic!("expected RegExp constructor expression");
    };

    assert_eq!(*span, qjs_ast::Span::new(0, 8));
    assert!(matches!(
        arguments.as_slice(),
        [
            Expr::Literal(Literal::String { value: pattern, .. }),
            Expr::Literal(Literal::String { value: flags, .. })
        ] if pattern == r#"\d{2}"# && flags == "g"
    ));
}

#[test]
fn parses_regexp_literal_with_comma() {
    let script = parse_script(r#"/,/;"#).expect("source should parse");
    let [Stmt::Expr(Expr::New { arguments, .. })] = script.body.as_slice() else {
        panic!("expected RegExp constructor expression");
    };

    assert!(matches!(
        arguments.as_slice(),
        [Expr::Literal(Literal::String { value, .. })] if value == ","
    ));
}

#[test]
fn parses_regexp_literal_with_space() {
    let script = parse_script(r#"/ /;"#).expect("source should parse");
    let [Stmt::Expr(Expr::New { arguments, .. })] = script.body.as_slice() else {
        panic!("expected RegExp constructor expression");
    };

    assert!(matches!(
        arguments.as_slice(),
        [Expr::Literal(Literal::String { value, .. })] if value == " "
    ));
}

#[test]
fn parses_date_format_regexp_literal_smoke() {
    let source = r#"/^(Sun|Mon) [0-9]{2}:[0-9]{2} GMT[+-][0-9]{4}( \(.+\))?$/;"#;
    let script = parse_script(source).expect("source should parse");
    let [Stmt::Expr(Expr::New { arguments, .. })] = script.body.as_slice() else {
        panic!("expected RegExp constructor expression");
    };

    assert!(matches!(
        arguments.as_slice(),
        [Expr::Literal(Literal::String { value, .. })]
            if value == r#"^(Sun|Mon) [0-9]{2}:[0-9]{2} GMT[+-][0-9]{4}( \(.+\))?$"#
    ));
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
