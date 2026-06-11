use qjs_ast::{ClassElement, Expr, ForInLeft, MethodKind, Stmt};

use crate::parse_script;

/// Extracts the single function declaration's async/generator flags and body.
fn sole_function_decl(source: &str) -> (bool, bool, Vec<Stmt>) {
    let script = parse_script(source).expect("source should parse");
    let [
        Stmt::FunctionDecl {
            is_async,
            is_generator,
            body,
            ..
        },
    ] = script.body.as_slice()
    else {
        panic!("expected a single function declaration");
    };
    (*is_async, *is_generator, body.clone())
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
fn parses_async_function_declaration() {
    let (is_async, is_generator, body) = sole_function_decl("async function f() { await 1; }");
    assert!(is_async);
    assert!(!is_generator);
    assert!(matches!(body.as_slice(), [Stmt::Expr(Expr::Await { .. })]));
}

#[test]
fn non_async_function_flag_is_false() {
    let (is_async, _, _) = sole_function_decl("function f() { return 1; }");
    assert!(!is_async);
}

#[test]
fn parses_async_generator_declaration() {
    // `async function*` sets both flags at the AST level.
    let (is_async, is_generator, _) = sole_function_decl("async function* f() { yield await 1; }");
    assert!(is_async);
    assert!(is_generator);
}

#[test]
fn parses_async_function_expression() {
    let Expr::Function {
        is_async,
        constructable,
        name,
        ..
    } = sole_expr("(async function f() { await 1; });")
    else {
        panic!("expected async function expression");
    };
    assert!(is_async);
    assert!(!constructable, "async functions are not constructable");
    assert_eq!(name.as_deref(), Some("f"));
}

#[test]
fn parses_async_arrow_single_identifier() {
    let Expr::Function {
        is_async,
        lexical_this,
        body,
        ..
    } = sole_expr("(async x => await x);")
    else {
        panic!("expected async arrow function");
    };
    assert!(is_async);
    assert!(lexical_this);
    assert!(matches!(
        body.as_slice(),
        [Stmt::Return {
            argument: Some(Expr::Await { .. }),
            ..
        }]
    ));
}

#[test]
fn parses_async_arrow_parenthesized() {
    let Expr::Function {
        is_async, params, ..
    } = sole_expr("(async (a, b) => { await a; });")
    else {
        panic!("expected async arrow function");
    };
    assert!(is_async);
    assert_eq!(params.positional.len(), 2);
}

#[test]
fn parses_async_no_parameters_arrow() {
    let Expr::Function { is_async, .. } = sole_expr("(async () => 1);") else {
        panic!("expected async arrow function");
    };
    assert!(is_async);
}

#[test]
fn async_call_is_not_an_arrow() {
    // `async(x)` with no `=>` is a call to a function named `async`.
    let Expr::Call { callee, .. } = sole_expr("async(x);") else {
        panic!("expected call expression");
    };
    assert!(matches!(
        callee.as_ref(),
        Expr::Identifier { name, .. } if name == "async"
    ));
}

#[test]
fn async_as_identifier_assignment() {
    let script = parse_script("async = 1;").expect("source should parse");
    let [Stmt::Expr(Expr::Assignment { target, .. })] = script.body.as_slice() else {
        panic!("expected assignment to async identifier");
    };
    assert!(matches!(
        target,
        qjs_ast::AssignmentTarget::Identifier { name, .. } if name == "async"
    ));
}

#[test]
fn async_as_property_name() {
    // `{ async: 1 }` is a data property named async, not an async method.
    let Expr::Object { properties, .. } = sole_expr("({ async: 1 });") else {
        panic!("expected object literal");
    };
    assert_eq!(properties.len(), 1);
    assert!(matches!(
        &properties[0].key,
        qjs_ast::ObjectPropertyKey::Literal(name) if name == "async"
    ));
    assert!(matches!(
        &properties[0].value,
        Expr::Literal(qjs_ast::Literal::Number { .. })
    ));
}

#[test]
fn async_shorthand_method_named_async() {
    // `{ async() {} }` is a method named async (no following name token), not
    // an async method.
    let Expr::Object { properties, .. } = sole_expr("({ async() {} });") else {
        panic!("expected object literal");
    };
    let Expr::Function { is_async, name, .. } = &properties[0].value else {
        panic!("expected method value");
    };
    assert!(!is_async);
    assert_eq!(name.as_deref(), Some("async"));
}

#[test]
fn parses_object_async_method() {
    let Expr::Object { properties, .. } = sole_expr("({ async m() { await 1; } });") else {
        panic!("expected object literal");
    };
    let Expr::Function { is_async, name, .. } = &properties[0].value else {
        panic!("expected method value");
    };
    assert!(*is_async);
    assert_eq!(name.as_deref(), Some("m"));
}

#[test]
fn parses_object_async_generator_method() {
    let Expr::Object { properties, .. } = sole_expr("({ async *m() { yield await 1; } });") else {
        panic!("expected object literal");
    };
    let Expr::Function {
        is_async,
        is_generator,
        ..
    } = &properties[0].value
    else {
        panic!("expected method value");
    };
    assert!(*is_async);
    assert!(*is_generator);
}

#[test]
fn parses_class_async_methods() {
    let script = parse_script(
        "class C { async m() { await 1; } static async s() { await 2; } async *g() { yield await 3; } async #p() { await 4; } }",
    )
    .expect("source should parse");
    let [Stmt::ClassDecl { body, .. }] = script.body.as_slice() else {
        panic!("expected class declaration");
    };
    let flags: Vec<(bool, bool)> = body
        .elements
        .iter()
        .filter_map(|element| match element {
            ClassElement::Method(member) => {
                let Expr::Function {
                    is_async,
                    is_generator,
                    ..
                } = &member.value
                else {
                    panic!("expected method function value");
                };
                assert_eq!(member.kind, MethodKind::Method);
                Some((*is_async, *is_generator))
            }
            ClassElement::Field(_) | ClassElement::StaticBlock(_) => None,
        })
        .collect();
    assert_eq!(
        flags,
        [(true, false), (true, false), (true, true), (true, false)]
    );
}

#[test]
fn async_getter_and_setter_are_errors() {
    // `get`/`set` cannot be async.
    assert!(parse_script("({ async get x() {} });").is_err());
    assert!(parse_script("class C { async get x() {} }").is_err());
    assert!(parse_script("class C { async set x(v) {} }").is_err());
}

#[test]
fn async_class_constructor_is_error() {
    assert!(parse_script("class C { async constructor() {} }").is_err());
}

#[test]
fn await_unary_precedence() {
    // `await a + b` parses as `(await a) + b`: await binds tighter than `+`.
    let (_, _, body) = sole_function_decl("async function f() { await a + b; }");
    let [Stmt::Expr(Expr::Binary { left, op, .. })] = body.as_slice() else {
        panic!("expected binary expression");
    };
    assert_eq!(*op, qjs_ast::BinaryOp::Add);
    assert!(matches!(left.as_ref(), Expr::Await { .. }));
}

#[test]
fn await_nests_right_associatively() {
    // `await await x` parses as `await (await x)`.
    let (_, _, body) = sole_function_decl("async function f() { await await x; }");
    let [Stmt::Expr(Expr::Await { argument, .. })] = body.as_slice() else {
        panic!("expected await expression");
    };
    assert!(matches!(argument.as_ref(), Expr::Await { .. }));
}

#[test]
fn await_in_arrow_inside_async_is_await_expression() {
    // Arrows inherit the enclosing async context.
    let (_, _, body) = sole_function_decl("async function f() { var g = () => await 1; }");
    let [Stmt::VarDecl { declarations, .. }] = body.as_slice() else {
        panic!("expected var declaration of arrow");
    };
    let Some(Expr::Function {
        body: arrow_body,
        is_async,
        ..
    }) = &declarations[0].init
    else {
        panic!("expected arrow initializer");
    };
    assert!(!is_async, "the inner arrow is not itself async");
    assert!(matches!(
        arrow_body.as_slice(),
        [Stmt::Return {
            argument: Some(Expr::Await { .. }),
            ..
        }]
    ));
}

#[test]
fn await_is_identifier_in_sloppy_non_async_code() {
    // Outside an async function, in sloppy mode, `await` is a normal identifier.
    let script = parse_script("var await = 1; await;").expect("sloppy await identifier parses");
    let [
        Stmt::VarDecl { .. },
        Stmt::Expr(Expr::Identifier { name, .. }),
    ] = script.body.as_slice()
    else {
        panic!("expected var declaration then await identifier reference");
    };
    assert_eq!(name, "await");

    // `await` is a legal parameter name in a non-async function.
    assert!(parse_script("function f(await) { return await; }").is_ok());
}

#[test]
fn await_resets_in_nested_ordinary_function() {
    // A non-async function nested in an async function resets the await
    // context, so `await` is an identifier again.
    let script = parse_script("async function f() { function inner(await) { return await; } }")
        .expect("nested ordinary function should reset await context");
    assert!(matches!(
        script.body.as_slice(),
        [Stmt::FunctionDecl { .. }]
    ));
}

#[test]
fn await_in_async_parameter_default_is_error() {
    assert!(
        parse_script("async function f(a = await 1) {}").is_err(),
        "await in an async function parameter default is an early error"
    );
}

#[test]
fn await_as_binding_in_async_is_error() {
    assert!(parse_script("async function f(await) {}").is_err());
    assert!(parse_script("async function f() { var await = 1; }").is_err());
}

#[test]
fn async_function_declaration_named_await_at_top_level_is_allowed() {
    // The BindingIdentifier of an async function *declaration* is checked
    // against the enclosing Await context. At the top level of a script that
    // context is sloppy, so `await` is a legal name (Test262
    // language/expressions/await/await-BindingIdentifier-in-global).
    let script =
        parse_script("async function await() { return 1 }").expect("top-level name `await` is ok");
    let [Stmt::FunctionDecl { name, is_async, .. }] = script.body.as_slice() else {
        panic!("expected an async function declaration named await");
    };
    assert_eq!(name, "await");
    assert!(is_async);
}

#[test]
fn async_function_expression_named_await_is_error() {
    // An async function *expression*'s own name uses `[+Await]`, so `await` is
    // never a legal name even outside an enclosing async context.
    assert!(parse_script("(async function await() {});").is_err());
}

#[test]
fn function_named_await_nested_in_async_is_error() {
    // Inside an async function body the enclosing Await context makes `await`
    // an illegal binding name, including for nested ordinary functions
    // (Test262 language/expressions/await/await-BindingIdentifier-nested).
    assert!(parse_script("async function foo() { function await() {} }").is_err());
    assert!(parse_script("async function foo() { async function await() {} }").is_err());
}

#[test]
fn await_as_label_in_async_is_error() {
    assert!(parse_script("async function f() { await: 1; }").is_err());
    // A label `await` in sloppy non-async code is a valid label.
    assert!(parse_script("function f() { await: 1; }").is_ok());
    assert!(parse_script("await: 1;").is_ok());
}

#[test]
fn async_function_strict_name_eval_or_arguments_is_error() {
    // In strict code an async function named `eval`/`arguments` is a
    // SyntaxError, matching the rule for ordinary function declarations
    // (Test262 .../async-function/early-errors-declaration-binding-identifier-*).
    assert!(parse_script("\"use strict\"; async function eval() {}").is_err());
    assert!(parse_script("\"use strict\"; async function arguments() {}").is_err());
    assert!(parse_script("\"use strict\"; function eval() {}").is_err());
    assert!(parse_script("\"use strict\"; function arguments() {}").is_err());
    // Sloppy mode still allows these names.
    assert!(parse_script("async function eval() {}").is_ok());
    assert!(parse_script("function arguments() {}").is_ok());
}

#[test]
fn parameter_conflicting_with_body_lexical_declaration_is_error() {
    // BoundNames of FormalParameters may not also occur in the
    // LexicallyDeclaredNames of the body (Test262
    // .../async-function/early-errors-declaration-formals-body-duplicate).
    assert!(parse_script("async function foo(bar) { let bar; }").is_err());
    assert!(parse_script("function foo(bar) { let bar; }").is_err());
    assert!(parse_script("function foo(bar) { const bar = 1; }").is_err());
    assert!(parse_script("function foo(bar) { class bar {} }").is_err());
    // `var` and function declarations are var-scoped, so they do not conflict.
    assert!(parse_script("function foo(bar) { var bar; }").is_ok());
    assert!(parse_script("function foo(bar) { function bar() {} }").is_ok());
}

#[test]
fn line_terminator_between_async_and_function_is_identifier() {
    // ASI: `async` on its own line is an identifier expression statement, then
    // `function f() {}` is a separate (non-async) declaration.
    let script = parse_script("async\nfunction f() {}").expect("source should parse");
    let [
        Stmt::Expr(Expr::Identifier { name, .. }),
        Stmt::FunctionDecl { is_async, .. },
    ] = script.body.as_slice()
    else {
        panic!("expected async identifier then function declaration");
    };
    assert_eq!(name, "async");
    assert!(!is_async);
}

#[test]
fn line_terminator_before_async_arrow_parameters_is_error() {
    // No line terminator allowed between `async` and arrow parameters.
    assert!(parse_script("(async\nx => x);").is_err());
}

#[test]
fn parses_for_await_of() {
    let (_, _, body) = sole_function_decl("async function f() { for await (const x of y) {} }");
    let [Stmt::ForOf { is_await, left, .. }] = body.as_slice() else {
        panic!("expected for-of statement");
    };
    assert!(is_await);
    assert!(matches!(left, ForInLeft::VarDecl { .. }));
}

#[test]
fn plain_for_of_is_not_await() {
    let (_, _, body) = sole_function_decl("async function f() { for (const x of y) {} }");
    let [Stmt::ForOf { is_await, .. }] = body.as_slice() else {
        panic!("expected for-of statement");
    };
    assert!(!is_await);
}

#[test]
fn for_await_outside_async_is_error() {
    // `await` is only the contextual for-await keyword inside an async function.
    assert!(parse_script("for await (const x of y) {}").is_err());
}

#[test]
fn for_await_with_for_in_is_error() {
    assert!(parse_script("async function f() { for await (const x in y) {} }").is_err());
}
