use qjs_ast::{ClassElement, Expr, MethodKind, ObjectPropertyKind, Stmt};

use crate::parse_script;

/// Extracts the single function declaration from a parsed script.
fn sole_function_decl(source: &str) -> (bool, Vec<Stmt>) {
    let script = parse_script(source).expect("source should parse");
    let [
        Stmt::FunctionDecl {
            is_generator, body, ..
        },
    ] = script.body.as_slice()
    else {
        panic!("expected a single function declaration");
    };
    (*is_generator, body.clone())
}

/// Extracts the single expression statement's expression.
fn sole_expr(source: &str) -> Expr {
    let script = parse_script(source).expect("source should parse");
    let [Stmt::Expr(expr)] = script.body.as_slice() else {
        panic!("expected a single expression statement");
    };
    expr.clone()
}

#[test]
fn parses_generator_declaration() {
    let (is_generator, body) = sole_function_decl("function* gen() { yield 1; }");
    assert!(is_generator);
    assert!(matches!(
        body.as_slice(),
        [Stmt::Expr(Expr::Yield {
            delegate: false,
            argument: Some(_),
            ..
        })]
    ));
}

#[test]
fn non_generator_declaration_flag_is_false() {
    let (is_generator, _) = sole_function_decl("function plain() { return 1; }");
    assert!(!is_generator);
}

#[test]
fn parses_named_generator_expression() {
    let Expr::Function {
        is_generator,
        constructable,
        name,
        ..
    } = sole_expr("(function* gen() { yield 1; });")
    else {
        panic!("expected generator function expression");
    };
    assert!(is_generator);
    assert!(!constructable, "generators are not constructable");
    assert_eq!(name.as_deref(), Some("gen"));
}

#[test]
fn parses_anonymous_generator_expression() {
    let Expr::Function {
        is_generator, name, ..
    } = sole_expr("(function* () { yield; });")
    else {
        panic!("expected generator function expression");
    };
    assert!(is_generator);
    assert_eq!(name, None);
}

#[test]
fn parses_object_generator_method() {
    let Expr::Object { properties, .. } = sole_expr("({ *gen() { yield 1; } });") else {
        panic!("expected object literal");
    };
    assert_eq!(properties.len(), 1);
    assert_eq!(properties[0].kind, ObjectPropertyKind::Data);
    let Expr::Function { is_generator, .. } = &properties[0].value else {
        panic!("expected method value to be a function");
    };
    assert!(*is_generator);
}

#[test]
fn parses_class_generator_methods() {
    let script =
        parse_script("class C { *m() { yield 1; } static *s() { yield 2; } *#p() { yield 3; } }")
            .expect("source should parse");
    let [Stmt::ClassDecl { body, .. }] = script.body.as_slice() else {
        panic!("expected class declaration");
    };
    let generators: Vec<bool> = body
        .elements
        .iter()
        .filter_map(|element| match element {
            ClassElement::Method(member) => {
                let Expr::Function { is_generator, .. } = &member.value else {
                    panic!("expected method function value");
                };
                assert_eq!(member.kind, MethodKind::Method);
                Some(*is_generator)
            }
            ClassElement::Field(_) | ClassElement::StaticBlock(_) => None,
        })
        .collect();
    assert_eq!(generators, [true, true, true]);
}

#[test]
fn yield_precedence_forms() {
    // Bare yield with no operand before the closing brace.
    let (_, body) = sole_function_decl("function* g() { yield; }");
    assert!(matches!(
        body.as_slice(),
        [Stmt::Expr(Expr::Yield {
            argument: None,
            delegate: false,
            ..
        })]
    ));

    // yield with an operand.
    let (_, body) = sole_function_decl("function* g() { yield x; }");
    assert!(matches!(
        body.as_slice(),
        [Stmt::Expr(Expr::Yield {
            argument: Some(_),
            delegate: false,
            ..
        })]
    ));

    // yield* delegation always carries an operand.
    let (_, body) = sole_function_decl("function* g() { yield* x; }");
    assert!(matches!(
        body.as_slice(),
        [Stmt::Expr(Expr::Yield {
            argument: Some(_),
            delegate: true,
            ..
        })]
    ));
}

#[test]
fn yield_is_assignment_expression() {
    // `x = yield`: the yield is the assigned value, not vice versa.
    let (_, body) = sole_function_decl("function* g() { x = yield; }");
    let [Stmt::Expr(Expr::Assignment { value, .. })] = body.as_slice() else {
        panic!("expected assignment with yield value");
    };
    assert!(matches!(value.as_ref(), Expr::Yield { argument: None, .. }));
}

#[test]
fn yield_nests_right_associatively() {
    // `yield yield 1` parses as `yield (yield 1)`.
    let (_, body) = sole_function_decl("function* g() { yield yield 1; }");
    let [
        Stmt::Expr(Expr::Yield {
            argument: Some(inner),
            delegate: false,
            ..
        }),
    ] = body.as_slice()
    else {
        panic!("expected nested yield");
    };
    assert!(matches!(
        inner.as_ref(),
        Expr::Yield {
            argument: Some(_),
            delegate: false,
            ..
        }
    ));
}

#[test]
fn yield_no_line_terminator_before_star() {
    // A line terminator between `yield` and `*` is a syntax error.
    assert!(parse_script("function* g() { yield\n* x; }").is_err());
}

#[test]
fn yield_before_newline_has_no_operand() {
    // ASI-like rule: a line terminator after `yield` ends the operand-less form,
    // so the following expression is a separate statement.
    let (_, body) = sole_function_decl("function* g() { yield\nx; }");
    assert!(matches!(
        body.as_slice(),
        [
            Stmt::Expr(Expr::Yield { argument: None, .. }),
            Stmt::Expr(Expr::Identifier { .. }),
        ]
    ));
}

#[test]
fn yield_is_identifier_in_sloppy_non_generator_code() {
    // Outside a generator, in sloppy mode, `yield` is a normal identifier.
    let script = parse_script("var yield = 1; yield;").expect("sloppy yield identifier parses");
    let [
        Stmt::VarDecl { .. },
        Stmt::Expr(Expr::Identifier { name, .. }),
    ] = script.body.as_slice()
    else {
        panic!("expected var declaration then yield identifier reference");
    };
    assert_eq!(name, "yield");

    // Inside a non-generator function nested in a generator, `yield` is again an
    // identifier (the inner function resets the yield context).
    let script = parse_script("function* g() { function inner() { return yield; } }")
        .expect("nested non-generator function should reset yield context");
    assert!(matches!(
        script.body.as_slice(),
        [Stmt::FunctionDecl { .. }]
    ));
}

#[test]
fn yield_in_arrow_inside_generator_is_yield_expression() {
    // Arrows inherit the enclosing generator's yield context.
    let (_, body) = sole_function_decl("function* g() { var f = () => yield 1; }");
    let [Stmt::VarDecl { declarations, .. }] = body.as_slice() else {
        panic!("expected var declaration of arrow function");
    };
    let Some(Expr::Function {
        body: arrow_body,
        is_generator,
        ..
    }) = &declarations[0].init
    else {
        panic!("expected arrow function initializer");
    };
    assert!(!is_generator, "arrows are never generators themselves");
    assert!(matches!(
        arrow_body.as_slice(),
        [Stmt::Return {
            argument: Some(Expr::Yield {
                argument: Some(_),
                ..
            }),
            ..
        }]
    ));
}

#[test]
fn yield_in_generator_parameter_default_is_error() {
    assert!(
        parse_script("function* g(a = yield) {}").is_err(),
        "yield in a generator parameter default is an early error"
    );
}

#[test]
fn strict_generator_named_yield_is_error() {
    assert!(parse_script("\"use strict\"; function* yield() {}").is_err());
    assert!(parse_script("\"use strict\"; (function* yield() {});").is_err());
}

#[test]
fn generator_expression_named_yield_is_error_even_in_sloppy() {
    // A generator *expression*'s own name uses `[+Yield]`, so `yield` is never
    // a legal name regardless of strict mode.
    assert!(parse_script("(function* yield() {});").is_err());
}

#[test]
fn function_named_yield_nested_in_generator_is_error() {
    // The enclosing Yield context bans `yield` as a binding name, including for
    // nested ordinary function declarations.
    assert!(parse_script("function* g() { function yield() {} }").is_err());
}

#[test]
fn yield_as_label_in_generator_is_error() {
    assert!(parse_script("function* g() { yield: 1; }").is_err());
    // `yield` is also reserved as a label in strict mode.
    assert!(parse_script("\"use strict\"; yield: 1;").is_err());
    // In sloppy non-generator code, `yield` is a valid label.
    assert!(parse_script("yield: 1;").is_ok());
    assert!(parse_script("function f() { yield: 1; }").is_ok());
}

#[test]
fn class_constructor_may_not_be_generator() {
    assert!(parse_script("class C { *constructor() {} }").is_err());
}

#[test]
fn accessors_may_not_be_generators() {
    // `get`/`set` cannot be combined with a generator marker; `*get` treats
    // `get` as the method name, so this is a plain generator method named `get`.
    let script = parse_script("({ *get() { yield 1; } });").expect("source should parse");
    let [Stmt::Expr(Expr::Object { properties, .. })] = script.body.as_slice() else {
        panic!("expected object literal");
    };
    let Expr::Function {
        is_generator, name, ..
    } = &properties[0].value
    else {
        panic!("expected method function value");
    };
    assert!(*is_generator);
    assert_eq!(name.as_deref(), Some("get"));
}
