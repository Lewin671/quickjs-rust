use qjs_ast::{
    ArrayElement, AssignmentTarget, CallArgument, Expr, FunctionParams, Literal, MemberProperty,
    ObjectPropertyKey, ObjectPropertyKind, Stmt,
};

use crate::{EvalParseContext, parse_direct_eval_script, parse_module, parse_script};

fn positional_names(params: &FunctionParams) -> Vec<String> {
    params
        .positional
        .iter()
        .flat_map(|element| element.binding.names())
        .collect()
}

fn rest_names(params: &FunctionParams) -> Vec<String> {
    params
        .rest
        .as_deref()
        .map(qjs_ast::BindingPattern::names)
        .unwrap_or_default()
}

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
    assert_eq!(positional_names(params), ["a", "b"]);
    assert_eq!(arguments.len(), 2);

    let script = parse_script("add(1, 2,);").expect("source should parse");
    let [Stmt::Expr(Expr::Call { arguments, .. })] = script.body.as_slice() else {
        panic!("expected function call with trailing comma");
    };
    assert_eq!(arguments.len(), 2);

    let script = parse_script("add(0, ...values, 3);").expect("source should parse");
    let [Stmt::Expr(Expr::Call { arguments, .. })] = script.body.as_slice() else {
        panic!("expected function call with spread arguments");
    };
    assert_eq!(arguments.len(), 3);
    assert!(matches!(arguments[1], CallArgument::Spread(_)));

    let script = parse_script("let f = function named(value) { return value; }; f(1);")
        .expect("source should parse");
    let [
        Stmt::VarDecl { declarations, .. },
        Stmt::Expr(Expr::Call { .. }),
    ] = script.body.as_slice()
    else {
        panic!("expected function expression assignment followed by call");
    };
    let Some(Expr::Function {
        name,
        params,
        lexical_this,
        lexical_arguments,
        ..
    }) = &declarations[0].init
    else {
        panic!("expected function expression initializer");
    };
    assert_eq!(name.as_deref(), Some("named"));
    assert_eq!(positional_names(params), ["value"]);
    assert!(!lexical_this);
    assert!(!lexical_arguments);

    let script = parse_script("let f = (a, b) => a + b;").expect("source should parse");
    let [Stmt::VarDecl { declarations, .. }] = script.body.as_slice() else {
        panic!("expected arrow function declaration");
    };
    let Some(Expr::Function {
        name,
        params,
        constructable,
        lexical_this,
        lexical_arguments,
        body,
        ..
    }) = &declarations[0].init
    else {
        panic!("expected arrow function initializer");
    };
    assert_eq!(name, &None);
    assert_eq!(positional_names(params), ["a", "b"]);
    assert!(!constructable);
    assert!(lexical_this);
    assert!(lexical_arguments);
    assert!(matches!(body.as_slice(), [Stmt::Return { .. }]));

    let script = parse_script("let f = value => { return value; };").expect("source should parse");
    let [Stmt::VarDecl { declarations, .. }] = script.body.as_slice() else {
        panic!("expected single-parameter arrow function declaration");
    };
    let Some(Expr::Function {
        params,
        body,
        lexical_this,
        lexical_arguments,
        ..
    }) = &declarations[0].init
    else {
        panic!("expected arrow function initializer");
    };
    assert_eq!(positional_names(params), ["value"]);
    assert!(lexical_this);
    assert!(lexical_arguments);
    assert!(matches!(body.as_slice(), [Stmt::Return { .. }]));

    let script =
        parse_script("function trailing(a, b,) { return a + b; }").expect("source should parse");
    let [Stmt::FunctionDecl { params, .. }] = script.body.as_slice() else {
        panic!("expected function declaration");
    };
    assert_eq!(positional_names(params), ["a", "b"]);

    let script = parse_script("let trailing = (a, b,) => a + b;").expect("source should parse");
    let [Stmt::VarDecl { declarations, .. }] = script.body.as_slice() else {
        panic!("expected arrow function declaration");
    };
    let Some(Expr::Function { params, .. }) = &declarations[0].init else {
        panic!("expected arrow function initializer");
    };
    assert_eq!(positional_names(params), ["a", "b"]);
}

#[test]
fn parses_default_parameters() {
    let script =
        parse_script("function pick(a, b = 2) { return b; }").expect("source should parse");
    let [Stmt::FunctionDecl { params, .. }] = script.body.as_slice() else {
        panic!("expected function declaration");
    };
    assert_eq!(positional_names(params), ["a", "b"]);
    assert_eq!(params.length(), 1);
    assert!(params.default_at(0).is_none());
    assert!(matches!(
        params.default_at(1),
        Some(Expr::Literal(Literal::Number { raw, .. })) if raw == "2"
    ));

    let script = parse_script("let pick = (a, b = a + 1,) => b;").expect("source should parse");
    let [Stmt::VarDecl { declarations, .. }] = script.body.as_slice() else {
        panic!("expected arrow function declaration");
    };
    let Some(Expr::Function { params, .. }) = &declarations[0].init else {
        panic!("expected arrow function initializer");
    };
    assert_eq!(positional_names(params), ["a", "b"]);
    assert_eq!(params.length(), 1);
    assert!(params.default_at(1).is_some());
}

#[test]
fn rejects_strict_function_body_with_default_parameters() {
    assert!(parse_script("let pick = (value = 1) => { 'use strict'; return value; };").is_err());
    assert!(parse_script("function pick(value = 1) { 'use strict'; return value; }").is_err());
}

#[test]
fn parses_rest_parameters() {
    let script = parse_script("function collect(first, ...rest) { return rest; }")
        .expect("source should parse");
    let [Stmt::FunctionDecl { params, .. }] = script.body.as_slice() else {
        panic!("expected function declaration");
    };
    assert_eq!(positional_names(params), ["first"]);
    assert_eq!(rest_names(params), ["rest"]);

    let script = parse_script("let collect = (...rest) => rest;").expect("source should parse");
    let [Stmt::VarDecl { declarations, .. }] = script.body.as_slice() else {
        panic!("expected arrow function declaration");
    };
    let Some(Expr::Function {
        params,
        lexical_this,
        lexical_arguments,
        ..
    }) = &declarations[0].init
    else {
        panic!("expected arrow function initializer");
    };
    assert!(params.positional.is_empty());
    assert_eq!(rest_names(params), ["rest"]);
    assert!(lexical_this);
    assert!(lexical_arguments);

    assert!(parse_script("function collect(...rest,) { return rest; }").is_err());
    assert!(parse_script("let collect = (...rest,) => rest;").is_err());
}

#[test]
fn rejects_duplicate_arrow_parameters() {
    assert!(parse_script("let duplicate = (value, value) => value;").is_err());
    assert!(parse_script("let duplicate = (value, ...value) => value;").is_err());

    let script =
        parse_script("function duplicate(value, value) { return value; }").expect("source");
    let [Stmt::FunctionDecl { params, .. }] = script.body.as_slice() else {
        panic!("expected function declaration");
    };
    assert_eq!(positional_names(params), ["value", "value"]);
}

#[test]
fn rejects_restricted_arrow_parameter_names_in_strict_mode() {
    assert!(parse_script("\"use strict\"; let arrow = arguments => 1;").is_err());
    assert!(parse_script("\"use strict\"; let arrow = (eval) => 1;").is_err());
    assert!(parse_script("\"use strict\"; let arrow = (value, ...yield) => value;").is_err());

    parse_script("let arrow = (arguments, eval, ...yield) => arguments;")
        .expect("non-strict arrow parameter names should parse");
    assert!(
        parse_script("\"use strict\"; function ordinary(arguments, eval) { return arguments; }")
            .is_err()
    );
    parse_script("function ordinary(arguments, eval) { return arguments; }")
        .expect("non-strict function parameter names should parse");
}

#[test]
fn rejects_reserved_arrow_parameter_names() {
    assert!(parse_script("let arrow = enum => 1;").is_err());
    assert!(parse_script("let arrow = (value, ...enum) => value;").is_err());
    assert!(parse_script("\"use strict\"; let arrow = package => 1;").is_err());
    assert!(parse_script("\"use strict\"; let arrow = (value, private) => value;").is_err());

    parse_script("let arrow = package => package;")
        .expect("strict-only future reserved words should parse outside strict mode");
    // `enum` is an unconditionally reserved word, so it is rejected even in
    // ordinary function parameters.
    assert!(parse_script("function ordinary(enum, package) { return package; }").is_err());
    // `package` is only reserved in strict mode, so it parses here.
    parse_script("function ordinary(package) { return package; }")
        .expect("strict-only future reserved words should parse outside strict mode");
}

#[test]
fn rejects_line_terminator_before_arrow() {
    assert!(parse_script("let arrow = value\n=> value;").is_err());
    assert!(parse_script("let arrow = ()\n=> {};").is_err());
    assert!(parse_script("let arrow = (value)\r\n=> value;").is_err());
    assert!(parse_script("let arrow = (value)\u{2028}=> value;").is_err());

    parse_script("let arrow = value => value;").expect("same-line arrow should parse");
    parse_script("let arrow = () => {};").expect("same-line arrow block body should parse");
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

    let script = parse_script("new Point(...coords);").expect("source should parse");
    let [Stmt::Expr(Expr::New { arguments, .. })] = script.body.as_slice() else {
        panic!("expected spread new expression statement");
    };
    assert_eq!(arguments.len(), 1);
    assert!(matches!(arguments[0], CallArgument::Spread(_)));
}

#[test]
fn rejects_direct_await_as_new_operand_in_module() {
    assert!(parse_module("new await;").is_err());
}

#[test]
fn parses_parenthesized_dynamic_import_as_new_operand() {
    parse_script("new (import('module'));").expect("covered import call can be a new operand");
    parse_script("new (function() {}, import('module'));")
        .expect("covered sequence ending in import call can be a new operand");
    parse_script("new (import('module'), function C() {});")
        .expect("covered sequence with constructable final expression parses");

    assert!(parse_script("new import('module');").is_err());
}

#[test]
fn parses_new_target_meta_property() {
    let script = parse_script("function C() { return new.target; }").expect("source should parse");
    let [Stmt::FunctionDecl { body, .. }] = script.body.as_slice() else {
        panic!("expected function declaration");
    };
    let [
        Stmt::Return {
            argument: Some(Expr::NewTarget { .. }),
            ..
        },
    ] = body.as_slice()
    else {
        panic!("expected return new.target statement");
    };
}

#[test]
fn rejects_new_target_outside_function_context() {
    assert!(parse_script("new.target;").is_err());
    assert!(parse_script("() => new.target;").is_err());
}

#[test]
fn rejects_return_outside_function_body() {
    assert!(parse_script("return;").is_err());
    assert!(parse_direct_eval_script("return;", EvalParseContext::default()).is_err());
    assert!(
        parse_direct_eval_script(
            "return;",
            EvalParseContext {
                in_function: true,
                ..EvalParseContext::default()
            },
        )
        .is_err()
    );
    parse_script("function f() { return; }").expect("function body may return");
    parse_script("({ m() { return 1; } });").expect("method body may return");
}

#[test]
fn parses_direct_eval_with_caller_context() {
    let context = EvalParseContext {
        in_function: true,
        in_method: true,
        in_derived_constructor: false,
        in_field_initializer: true,
        ..EvalParseContext::default()
    };
    parse_direct_eval_script("new.target; () => super.x;", context.clone())
        .expect("direct eval inherits function and method contexts");
    assert!(parse_direct_eval_script("arguments;", context.clone()).is_err());
    assert!(parse_direct_eval_script("super();", context).is_err());
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
    assert!(
        elements
            .iter()
            .all(|element| matches!(element, ArrayElement::Expr(_)))
    );
    let script = parse_script("[1, , 3];").expect("source should parse");
    let [Stmt::Expr(Expr::Array { elements, .. })] = script.body.as_slice() else {
        panic!("expected one array expression");
    };
    assert_eq!(elements.len(), 3);
    assert!(matches!(elements[1], ArrayElement::Elision));
    let script = parse_script("[1, ...items, 3];").expect("source should parse");
    let [Stmt::Expr(Expr::Array { elements, .. })] = script.body.as_slice() else {
        panic!("expected one array expression");
    };
    assert!(matches!(elements[0], ArrayElement::Expr(_)));
    assert!(matches!(elements[1], ArrayElement::Spread(_)));
    assert!(matches!(elements[2], ArrayElement::Expr(_)));
}

#[test]
fn parses_bigint_literal() {
    let script = parse_script("123n;").expect("source should parse");
    let [Stmt::Expr(Expr::Literal(Literal::BigInt { raw, span }))] = script.body.as_slice() else {
        panic!("expected BigInt literal expression");
    };
    assert_eq!(raw, "123");
    assert_eq!(*span, qjs_ast::Span::new(0, 4));
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
fn parses_template_literal_with_substitutions() {
    let script = parse_script("`a ${name} b ${1 + 2} c`;").expect("script should parse");
    let [
        Stmt::Expr(Expr::Template {
            parts, expressions, ..
        }),
    ] = script.body.as_slice()
    else {
        panic!("expected one template literal expression");
    };
    assert_eq!(
        parts,
        &vec!["a ".to_owned(), " b ".to_owned(), " c".to_owned()]
    );
    assert_eq!(expressions.len(), 2);
    assert!(matches!(expressions[0], Expr::Identifier { .. }));
    assert!(matches!(expressions[1], Expr::Binary { .. }));
}

#[test]
fn rejects_legacy_octal_escapes_in_strict_strings_and_templates() {
    assert!(parse_script("\"use strict\"; '\\07';").is_err());
    assert!(parse_script("\"use strict\"; `${'\\07'}`;").is_err());
    parse_script("'\\07';").expect("legacy octal escapes parse outside strict mode strings");
}

#[test]
fn rejects_legacy_octal_escapes_in_untagged_templates() {
    for source in ["`\\00`;", "`\\8`;", "`\\9`;", "`${'ok'}\\00`;"] {
        assert!(
            parse_script(source).is_err(),
            "expected untagged template to reject {source}"
        );
    }
    parse_script("tag`\\00`; tag`\\8`; tag`\\9`;")
        .expect("tagged templates preserve invalid escape sequences for the tag");
}

#[test]
fn rejects_annex_b_numeric_literals_in_strict_mode() {
    assert!(parse_script("\"use strict\"; 00;").is_err());
    assert!(parse_script("\"use strict\"; 010;").is_err());
    assert!(parse_script("\"use strict\"; 08;").is_err());
    assert!(parse_script("\"use strict\"; ({ 01: 1 });").is_err());
    assert!(parse_script("\"use strict\"; ({ 08: target } = value);").is_err());
    parse_script("00; 010; 08;").expect("Annex B numeric literals parse outside strict mode");

    let context = EvalParseContext {
        strict: true,
        ..EvalParseContext::default()
    };
    assert!(parse_direct_eval_script("010;", context).is_err());
}

#[test]
fn parses_tagged_template_literal() {
    let script = parse_script("tag`a ${value} b`;").expect("script should parse");
    let [
        Stmt::Expr(Expr::TaggedTemplate {
            tag,
            cooked,
            raw,
            expressions,
            ..
        }),
    ] = script.body.as_slice()
    else {
        panic!("expected one tagged template expression");
    };
    assert!(matches!(tag.as_ref(), Expr::Identifier { .. }));
    assert_eq!(cooked, &vec!["a ".to_owned(), " b".to_owned()]);
    assert_eq!(raw, &vec!["a ".to_owned(), " b".to_owned()]);
    assert_eq!(expressions.len(), 1);
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
        parse_script("({ return() { return 1; }, default: 2, class: 3, extends: 4, super: 5 });")
            .expect("source should parse");
    let [Stmt::Expr(Expr::Object { properties, .. })] = script.body.as_slice() else {
        panic!("expected object expression with keyword property names");
    };
    assert_eq!(
        properties[0].key,
        ObjectPropertyKey::Literal("return".to_owned())
    );
    assert_eq!(
        properties[1].key,
        ObjectPropertyKey::Literal("default".to_owned())
    );
    assert_eq!(
        properties[2].key,
        ObjectPropertyKey::Literal("class".to_owned())
    );
    assert_eq!(
        properties[3].key,
        ObjectPropertyKey::Literal("extends".to_owned())
    );
    assert_eq!(
        properties[4].key,
        ObjectPropertyKey::Literal("super".to_owned())
    );
    assert!(matches!(properties[0].value, Expr::Function { .. }));

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

    let script = parse_script("let source = {}; ({ ...source, after: 1 });")
        .expect("object spread should parse");
    let [_, Stmt::Expr(Expr::Object { properties, .. })] = script.body.as_slice() else {
        panic!("expected object expression with spread property");
    };
    assert_eq!(properties[0].kind, ObjectPropertyKind::Spread);
    assert!(matches!(
        properties[0].value,
        Expr::Identifier { ref name, .. } if name == "source"
    ));

    let script =
        parse_script("({ 999999999999999999n: true, 0xf() {} });").expect("source should parse");
    let [Stmt::Expr(Expr::Object { properties, .. })] = script.body.as_slice() else {
        panic!("expected object expression with BigInt property keys");
    };
    assert_eq!(
        properties[0].key,
        ObjectPropertyKey::Literal("999999999999999999".to_owned())
    );
    assert_eq!(
        properties[1].key,
        ObjectPropertyKey::Literal("15".to_owned())
    );
    parse_script("let { 1n: value } = { '1': 1 };")
        .expect("BigInt literal binding property names should parse");

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
    assert_eq!(positional_names(params), ["a".to_owned(), "b".to_owned()]);
    assert!(!constructable);

    let script = parse_script("({ keys: function* keys() { yield 2; yield 3; } });")
        .expect("source should parse");
    let [Stmt::Expr(Expr::Object { properties, .. })] = script.body.as_slice() else {
        panic!("expected object expression with generator function property");
    };
    let Expr::Function {
        name,
        body,
        constructable,
        is_generator,
        ..
    } = &properties[0].value
    else {
        panic!("expected generator function to parse as function expression");
    };
    assert_eq!(name.as_deref(), Some("keys"));
    assert!(!constructable);
    assert!(*is_generator);
    assert!(matches!(
        body.as_slice(),
        [
            Stmt::Expr(Expr::Yield {
                delegate: false,
                ..
            }),
            Stmt::Expr(Expr::Yield {
                delegate: false,
                ..
            }),
        ]
    ));

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

    let script =
        parse_script("({ set value(next) { this.seen = next; } });").expect("source should parse");
    let [Stmt::Expr(Expr::Object { properties, .. })] = script.body.as_slice() else {
        panic!("expected object expression with setter");
    };
    assert_eq!(
        properties[0].key,
        ObjectPropertyKey::Literal("value".to_owned())
    );
    assert_eq!(properties[0].kind, ObjectPropertyKind::Setter);
    let Expr::Function {
        name,
        params,
        constructable,
        ..
    } = &properties[0].value
    else {
        panic!("expected setter value to parse as function expression");
    };
    assert_eq!(name.as_deref(), Some("value"));
    assert_eq!(positional_names(params), ["next".to_owned()]);
    assert!(!constructable);

    let script = parse_script("({ set() { return 42; } });").expect("source should parse");
    let [Stmt::Expr(Expr::Object { properties, .. })] = script.body.as_slice() else {
        panic!("expected object expression with method named set");
    };
    assert_eq!(
        properties[0].key,
        ObjectPropertyKey::Literal("set".to_owned())
    );
    assert_eq!(properties[0].kind, ObjectPropertyKind::Data);
    assert!(parse_script("({ set value() {} });").is_err());
    assert!(parse_script("({ set value(a, b) {} });").is_err());
}

#[test]
fn rejects_duplicate_object_literal_proto_setters() {
    let error = parse_script("({ __proto__: null, other: null, '__proto__': null });")
        .expect_err("duplicate __proto__ colon data properties are an early error");
    assert!(error.message.contains("duplicate __proto__"));

    parse_script("({ __proto__: null, ['__proto__']: 1, __proto__() { return 2; } });")
        .expect("only colon data __proto__ properties participate in the duplicate check");
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
        [CallArgument::Expr(Expr::Literal(Literal::String { value, .. }))] if value == "."
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
        [CallArgument::Expr(Expr::Literal(Literal::String { value, span }))]
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
            CallArgument::Expr(Expr::Literal(Literal::String { value: pattern, span: pattern_span })),
            CallArgument::Expr(Expr::Literal(Literal::String { value: flags, span: flags_span }))
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
            CallArgument::Expr(Expr::Literal(Literal::String { value: pattern, .. })),
            CallArgument::Expr(Expr::Literal(Literal::String { value: flags, .. }))
        ] if pattern == r#"\d{2}"# && flags == "g"
    ));
}

#[test]
fn parses_regexp_literal_with_escaped_slash() {
    let script = parse_script(r#"/\//;"#).expect("source should parse");
    let [
        Stmt::Expr(Expr::New {
            arguments, span, ..
        }),
    ] = script.body.as_slice()
    else {
        panic!("expected RegExp constructor expression");
    };

    assert_eq!(*span, qjs_ast::Span::new(0, 4));
    assert!(matches!(
        arguments.as_slice(),
        [CallArgument::Expr(Expr::Literal(Literal::String { value, span }))]
            if value == r#"\/"# && *span == qjs_ast::Span::new(0, 4)
    ));
}

#[test]
fn parses_regexp_literal_with_braced_unicode_escape_and_u_flag() {
    let script = parse_script(r#"/\u{1d306}/u;"#).expect("source should parse");
    let [
        Stmt::Expr(Expr::New {
            arguments, span, ..
        }),
    ] = script.body.as_slice()
    else {
        panic!("expected RegExp constructor expression");
    };

    assert_eq!(*span, qjs_ast::Span::new(0, 12));
    assert!(matches!(
        arguments.as_slice(),
        [
            CallArgument::Expr(Expr::Literal(Literal::String { value: pattern, span: pattern_span })),
            CallArgument::Expr(Expr::Literal(Literal::String { value: flags, span: flags_span }))
        ] if pattern == r#"\u{1d306}"#
            && *pattern_span == qjs_ast::Span::new(0, 11)
            && flags == "u"
            && *flags_span == qjs_ast::Span::new(11, 12)
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
        [CallArgument::Expr(Expr::Literal(Literal::String { value, .. }))] if value == ","
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
        [CallArgument::Expr(Expr::Literal(Literal::String { value, .. }))] if value == " "
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
        [CallArgument::Expr(Expr::Literal(Literal::String { value, .. }))]
            if value == r#"^(Sun|Mon) [0-9]{2}:[0-9]{2} GMT[+-][0-9]{4}( \(.+\))?$"#
    ));
}

#[test]
fn parses_member_access() {
    let script = parse_script("items[0].length; item.class; item.extends; item.super; item.new;")
        .expect("source should parse");
    let [
        Stmt::Expr(Expr::Member {
            object, property, ..
        }),
        Stmt::Expr(Expr::Member {
            property: class_property,
            ..
        }),
        Stmt::Expr(Expr::Member {
            property: extends_property,
            ..
        }),
        Stmt::Expr(Expr::Member {
            property: super_property,
            ..
        }),
        Stmt::Expr(Expr::Member {
            property: new_property,
            ..
        }),
    ] = script.body.as_slice()
    else {
        panic!("expected member expressions");
    };
    assert_eq!(property, &MemberProperty::Named("length".to_owned()));
    assert_eq!(class_property, &MemberProperty::Named("class".to_owned()));
    assert_eq!(
        extends_property,
        &MemberProperty::Named("extends".to_owned())
    );
    assert_eq!(super_property, &MemberProperty::Named("super".to_owned()));
    assert_eq!(new_property, &MemberProperty::Named("new".to_owned()));
    assert!(matches!(object.as_ref(), Expr::Member { .. }));
}

#[test]
fn parses_binding_pattern_parameters() {
    let script = parse_script(
        "function pick({key, nested: {inner} = {}}, [first = 1, , third], ...rest) { return inner; }",
    )
    .expect("source should parse");
    let [Stmt::FunctionDecl { params, .. }] = script.body.as_slice() else {
        panic!("expected function declaration");
    };
    assert_eq!(positional_names(params), ["key", "inner", "first", "third"]);
    assert_eq!(rest_names(params), ["rest"]);
    assert_eq!(params.length(), 2);
    assert!(!params.is_simple());
}

#[test]
fn parses_arrow_binding_pattern_parameters() {
    let script = parse_script("let arrow = ({a = 1}, [b], ...[c, d]) => a + b + c + d;")
        .expect("source should parse");
    let [Stmt::VarDecl { declarations, .. }] = script.body.as_slice() else {
        panic!("expected variable declaration");
    };
    let Some(Expr::Function { params, .. }) = &declarations[0].init else {
        panic!("expected arrow function initializer");
    };
    assert_eq!(positional_names(params), ["a", "b"]);
    assert_eq!(rest_names(params), ["c", "d"]);
}

#[test]
fn parses_binding_pattern_rest_elements() {
    let script = parse_script("let [first, ...others] = xs, {key, ...extra} = obj;")
        .expect("source should parse");
    let [Stmt::VarDecl { declarations, .. }] = script.body.as_slice() else {
        panic!("expected variable declaration");
    };
    assert_eq!(declarations[0].binding.names(), ["first", "others"]);
    assert_eq!(declarations[1].binding.names(), ["key", "extra"]);
}

#[test]
fn rejects_duplicate_names_in_pattern_parameters() {
    assert!(parse_script("function pick([value], value) { return value; }").is_err());
    assert!(parse_script("function pick({value}, {value}) { return value; }").is_err());
    assert!(parse_script("function pick(value = 1, value) { return value; }").is_err());
}

#[test]
fn rejects_rest_parameter_defaults() {
    assert!(parse_script("function pick(...rest = []) { return rest; }").is_err());
    assert!(parse_script("let [a, ...rest = []] = xs;").is_err());
}

#[test]
fn rejects_strict_directive_with_pattern_parameters() {
    assert!(parse_script("function pick([value]) { \"use strict\"; return value; }").is_err());
    assert!(parse_script("function pick(...rest) { \"use strict\"; return rest; }").is_err());
}

#[test]
fn rejects_duplicate_method_parameters() {
    assert!(parse_script("var o = {m(a, a) {}};").is_err());
    assert!(parse_script("var o = {set s([v, v]) {}};").is_err());
    parse_script("function ordinary(a, a) {}").expect("sloppy simple duplicates stay legal");
    parse_script("var o = {m(a, b) {}};").expect("unique method parameters parse");
}
