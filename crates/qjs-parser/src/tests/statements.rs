use qjs_ast::{
    BinaryOp, BindingPattern, Expr, ForInLeft, ForInit, ObjectBindingPropertyKey, Stmt, VarKind,
};

use crate::parse_script;

#[test]
fn parses_variable_declaration() {
    let script = parse_script("let answer = 40 + 2, missing;").expect("source should parse");
    let [
        Stmt::VarDecl {
            kind, declarations, ..
        },
    ] = script.body.as_slice()
    else {
        panic!("expected one variable declaration");
    };
    assert_eq!(*kind, VarKind::Let);
    assert_eq!(declarations.len(), 2);
    assert_eq!(declarations[0].binding.names(), vec!["answer"]);
    assert!(matches!(
        declarations[0].init,
        Some(Expr::Binary {
            op: BinaryOp::Add,
            ..
        })
    ));
    assert_eq!(declarations[1].binding.names(), vec!["missing"]);
    assert!(declarations[1].init.is_none());
}

#[test]
fn parses_variable_declaration_binding_patterns() {
    let script = parse_script("const [first = 1, , third] = xs, {value, key: renamed = 2} = obj;")
        .expect("source should parse");
    let [Stmt::VarDecl { declarations, .. }] = script.body.as_slice() else {
        panic!("expected one variable declaration");
    };

    assert_eq!(declarations[0].binding.names(), vec!["first", "third"]);
    assert!(matches!(
        declarations[0].binding,
        BindingPattern::Array { .. }
    ));
    assert_eq!(declarations[1].binding.names(), vec!["value", "renamed"]);
    assert!(matches!(
        declarations[1].binding,
        BindingPattern::Object { .. }
    ));
}

#[test]
fn parses_computed_object_binding_property_names() {
    let script = parse_script("let { [key()]: value, plain } = obj;").expect("source should parse");
    let [Stmt::VarDecl { declarations, .. }] = script.body.as_slice() else {
        panic!("expected one variable declaration");
    };
    let BindingPattern::Object { properties, .. } = &declarations[0].binding else {
        panic!("expected object binding pattern");
    };

    assert!(matches!(
        &properties[0].key,
        ObjectBindingPropertyKey::Computed(Expr::Call { .. })
    ));
    assert_eq!(properties[1].key.as_literal(), Some("plain"));
    assert_eq!(declarations[0].binding.names(), vec!["value", "plain"]);
}

#[test]
fn rejects_const_without_initializer() {
    let error = parse_script("const answer;").expect_err("const should require initializer");
    assert_eq!(error.message, "const declarations require an initializer");
}

#[test]
fn rejects_destructuring_declaration_without_initializer() {
    let error =
        parse_script("var [answer];").expect_err("destructuring should require initializer");
    assert_eq!(
        error.message,
        "destructuring declarations require an initializer"
    );
}

#[test]
fn rejects_var_hoisted_from_nested_block_conflicting_with_lexical() {
    // A `var` in an inner block hoists to the enclosing block's
    // VarDeclaredNames, so it conflicts with a lexical name declared there.
    for source in [
        "{ { var f; } let f; }",
        "{ let f; { var f; } }",
        "{ { var f; } const f = 1; }",
        "{ { var f; } class f {} }",
        "{ { var f; } async function f() {} }",
        "{ if (1) var f; let f; }",
        "function g() { { var f; } let f; }",
    ] {
        let error = parse_script(source).expect_err("hoisted var must conflict");
        assert_eq!(
            error.message, "declaration `f` conflicts with a lexical declaration",
            "source: {source}"
        );
    }

    // A `var` that does not hoist into the lexical name's block (sibling block,
    // deeper block, or across a function/arrow boundary) is not a conflict.
    for source in [
        "{ { var f; } { let f; } }",
        "{ var f; { let f; } }",
        "{ let f; function g() { var f; } }",
        "{ let f; (() => { var f; }); }",
        "{ let f; } { var f; }",
    ] {
        parse_script(source).unwrap_or_else(|error| panic!("{source} should parse: {error:?}"));
    }
}

#[test]
fn rejects_import_and_export_as_binding_identifiers() {
    // `import`/`export` are reserved words and may not name a binding, including
    // their escaped spellings (which arrive as plain Identifier tokens).
    for source in [
        "var export = 1;",
        "let import = 1;",
        "function f(import) {}",
        "try {} catch (export) {}",
        "const { export } = {};",
    ] {
        let error = parse_script(source).expect_err("import/export binding must be rejected");
        assert!(
            error.message.contains("reserved word"),
            "source: {source}, got: {}",
            error.message
        );
    }

    // The arrow-parameter destructuring forms are also a SyntaxError (Test262
    // arrow-function/dstr/syntax-error-ident-ref-{export,import}-escaped),
    // though the object/arrow disambiguation may report it differently.
    for source in [
        "var x = ({ export }) => {};",
        "var x = ({ \\u0065xport }) => {};",
        "var x = ({ \\u0069mport }) => {};",
    ] {
        parse_script(source).expect_err("import/export arrow-param binding must be rejected");
    }

    // They remain valid as property names and identifier-like names that merely
    // contain the substring.
    for source in [
        "({ import: 1 });",
        "var o = {}; o.export;",
        "var x = ({ exporter }) => {};",
    ] {
        parse_script(source).unwrap_or_else(|error| panic!("{source} should parse: {error:?}"));
    }
}

#[test]
fn rejects_block_level_function_conflicting_with_var() {
    // Inside a block, a plain function declaration is a LexicallyDeclaredName,
    // so it conflicts with a same-named `var` (including a `var` hoisted from a
    // nested block). Annex B's function-as-var relaxation does not apply here.
    for source in [
        "{ var f; function f() {} }",
        "{ function f() {} var f; }",
        "{ { var f; } function f() {} }",
        "function g() { { function f() {} var f; } }",
    ] {
        let error = parse_script(source).expect_err("block function vs var must conflict");
        assert!(
            error
                .message
                .contains("conflicts with a lexical declaration"),
            "source: {source}, got: {}",
            error.message
        );
    }

    // Annex B keeps `var` + plain function legal at a function/script top level,
    // and two plain functions may share a name in any sloppy block.
    for source in [
        "var f; function f() {}",
        "function f() {} var f;",
        "function g() { var f; function f() {} }",
        "{ function f() {} function f() {} }",
    ] {
        parse_script(source).unwrap_or_else(|error| panic!("{source} should parse: {error:?}"));
    }
}

#[test]
fn rejects_yield_as_binding_identifier_inside_generator() {
    // `yield` is reserved inside a generator (parameters and body), so it may
    // not name a binding there even in sloppy code.
    for source in [
        "({ *m(yield) {} })",
        "({ *m() { var yield; } })",
        "({ async *m(yield) {} })",
        "function* g(yield) {}",
        "function* g() { let yield; }",
        "({ *m() { let [yield] = []; } })",
    ] {
        parse_script(source).expect_err("yield binding in generator must be rejected");
    }

    // Outside a generator (a plain method/function or top-level sloppy code) and
    // inside a nested ordinary function, `yield` is a legal binding name; a
    // `yield` expression inside the generator body is still fine.
    for source in [
        "({ m(yield) {} })",
        "function f(yield) {}",
        "let yield = 1;",
        "function* g() { function h() { var yield; } }",
        "({ *m() { yield 1; } })",
    ] {
        parse_script(source).unwrap_or_else(|error| panic!("{source} should parse: {error:?}"));
    }
}

#[test]
fn restricts_import_meta_to_module_goal() {
    use crate::parse_module;

    // `import.meta` is an early SyntaxError under the Script goal (a plain
    // script or a `Function` constructor body).
    for source in ["import.meta;", "x = import.meta", "() => import.meta"] {
        let error = parse_script(source).expect_err("import.meta is invalid in a script");
        assert_eq!(error.message, "`import.meta` is only valid in a module");
    }

    // Under the Module goal it parses as a meta-property.
    for source in ["import.meta;", "const u = import.meta.url;"] {
        parse_module(source).unwrap_or_else(|error| panic!("{source} should parse: {error:?}"));
    }

    // `import.foo` is never a valid meta-property.
    parse_module("import.foo;").expect_err("only import.meta is a valid meta-property");
}

#[test]
fn requires_semicolon_or_asi_between_statements() {
    let error = parse_script("var str = '''';").expect_err("adjacent strings need a separator");
    assert_eq!(error.message, "expected `;` or newline after statement");

    let error = parse_script("left right;").expect_err("adjacent expressions need a separator");
    assert_eq!(error.message, "expected `;` or newline after statement");

    parse_script("var str = ''\n'';").expect("newline should trigger ASI");
    parse_script("left\nright;").expect("newline should trigger ASI");
}

#[test]
fn parses_if_else_statement() {
    let script =
        parse_script("if (true) { let x = 1; } else { let x = 2; }").expect("source should parse");
    let [
        Stmt::If {
            consequent,
            alternate,
            ..
        },
    ] = script.body.as_slice()
    else {
        panic!("expected one if statement");
    };
    assert!(matches!(consequent.as_ref(), Stmt::Block { .. }));
    assert!(matches!(alternate.as_deref(), Some(Stmt::Block { .. })));
}

#[test]
fn parses_while_statement() {
    let script = parse_script("while (x < 3) { x = x + 1; }").expect("source should parse");
    let [Stmt::While { body, .. }] = script.body.as_slice() else {
        panic!("expected one while statement");
    };
    assert!(matches!(body.as_ref(), Stmt::Block { .. }));
}

#[test]
fn parses_do_while_statement() {
    let script = parse_script("do { x++; } while (x < 3);").expect("source should parse");
    let [Stmt::DoWhile { body, .. }] = script.body.as_slice() else {
        panic!("expected one do-while statement");
    };
    assert!(matches!(body.as_ref(), Stmt::Block { .. }));
}

#[test]
fn parses_for_statement() {
    let script =
        parse_script("for (var i = 0; i < 3; i = i + 1) { i; }").expect("source should parse");
    let [
        Stmt::For {
            init,
            test,
            update,
            body,
            ..
        },
    ] = script.body.as_slice()
    else {
        panic!("expected one for statement");
    };
    assert!(matches!(
        init,
        Some(ForInit::VarDecl { declarations, .. })
            if declarations.len() == 1 && declarations[0].binding.names() == vec!["i"]
    ));
    assert!(matches!(
        test,
        Some(Expr::Binary {
            op: BinaryOp::Lt,
            ..
        })
    ));
    assert!(matches!(update, Some(Expr::Assignment { .. })));
    assert!(matches!(body.as_ref(), Stmt::Block { .. }));
}

#[test]
fn parses_for_in_statement() {
    let script = parse_script("for (var key in object) { key; }").expect("source should parse");
    let [Stmt::ForIn { left, body, .. }] = script.body.as_slice() else {
        panic!("expected one for-in statement");
    };
    assert!(matches!(
        left,
        ForInLeft::VarDecl {
            binding: BindingPattern::Identifier { name, .. },
            ..
        } if name == "key"
    ));
    assert!(matches!(body.as_ref(), Stmt::Block { .. }));

    let script = parse_script("for (key in object) key;").expect("source should parse");
    let [Stmt::ForIn { left, .. }] = script.body.as_slice() else {
        panic!("expected one for-in statement");
    };
    assert!(matches!(left, ForInLeft::Target(_)));

    let script =
        parse_script("for (var key = init() in object) key;").expect("source should parse");
    let [Stmt::ForIn { left, .. }] = script.body.as_slice() else {
        panic!("expected one for-in statement");
    };
    assert!(matches!(
        left,
        ForInLeft::VarDecl {
            binding: BindingPattern::Identifier { name, .. },
            init: Some(_),
            ..
        } if name == "key"
    ));
    assert!(parse_script("\"use strict\"; for (var key = 0 in object) key;").is_err());
    parse_script("\"use strict\"; for (var key = 0; key < 1; key = key + 1) key;")
        .expect("strict var for-loop initializers should parse");
}

#[test]
fn validates_for_in_head_static_semantics() {
    for source in [
        "for (let [x, x] in {}) {}",
        "for (const [x, x] in {}) {}",
        "for (let x in {}) { var x; }",
        "for (const x in {}) { var x; }",
        "for (let x in {}) if (true) var x;",
        "\"use strict\"; for (let in {}) {}",
        "\"use strict\"; for (var eval in null) {}",
        "\"use strict\"; for (var arguments in null) {}",
        "\"use strict\"; function f() { for (var arguments in null) {} }",
    ] {
        parse_script(source).expect_err("for-in static semantics should reject source");
    }

    parse_script("for (var x in { attr: null }) { var x; }")
        .expect("var for-in head may be redeclared by the body");
    parse_script("for (let in {}) { }").expect("sloppy `let` is a valid for-in target");
    parse_script("var let, value; for (let in { key: 1 }) ;")
        .expect("sloppy `let` is a valid var binding and for-in target");
    parse_script("for (var x in null) let\nx = 1;")
        .expect("newline after sloppy `let` starts an expression statement body");

    parse_script("for (var x in null) let\n[a] = 0;")
        .expect_err("expression statements may not start with `let [`");
    parse_script("let\n[a] = 0;").expect_err("expression statements may not start with `let [`");
}

#[test]
fn parses_let_across_line_terminator_as_lexical_declaration_in_statement_lists() {
    parse_script("let\nlet;").expect_err("`let let` is a lexical declaration early error");
    parse_script("let\nlet = 1;")
        .expect_err("`let let = ...` is a lexical declaration early error");
    parse_script("function* f() { let\nyield 0; }")
        .expect_err("`let yield` in a generator is a lexical declaration early error");

    parse_script("for (var x in null) let\nx = 1;")
        .expect("single-statement bodies still allow sloppy `let` expression statements");
}

#[test]
fn validates_for_of_head_static_semantics() {
    for source in [
        // ForBinding bound names may not also be VarDeclaredNames of the body,
        // including the `using`/`await using` declaration forms.
        "for (using x of []) { var x; }",
        "async function f() { for (await using x of []) { var x; } }",
        // The for-of LeftHandSideExpression forbids a bare `let` and a leading
        // `async` identifier.
        "for (let of []) ;",
        "for (async of [1]) ;",
    ] {
        parse_script(source).expect_err("for-of static semantics should reject source");
    }

    // The same forms stay valid where the restriction does not apply.
    parse_script("for (using x of []) { var y; }").expect("non-conflicting using head is valid");
    parse_script("for (let in [1]) ;").expect("`let` is a valid for-in target");
    parse_script("for (async in [1]) ;").expect("`async` is a valid for-in target");
    parse_script("var obj = {}; for (obj.x of [1]) ;").expect("member for-of target is valid");
    parse_script("var async; for ((async) of [1]) ;")
        .expect("parenthesized `async` for-of target is valid");
    parse_script("for (let x of []) ;").expect("a `let` declaration head is valid");
}

#[test]
fn parses_for_of_statement() {
    let script =
        parse_script("for (const value of values) { value; }").expect("source should parse");
    let [Stmt::ForOf { left, body, .. }] = script.body.as_slice() else {
        panic!("expected one for-of statement");
    };
    assert!(matches!(
        left,
        ForInLeft::VarDecl {
            binding: BindingPattern::Identifier { name, .. },
            kind: VarKind::Const,
            ..
        } if name == "value"
    ));
    assert!(matches!(body.as_ref(), Stmt::Block { .. }));

    let script =
        parse_script("for (target.value of values) target.value;").expect("source should parse");
    let [Stmt::ForOf { left, .. }] = script.body.as_slice() else {
        panic!("expected one for-of statement");
    };
    assert!(matches!(left, ForInLeft::Target(_)));

    let script = parse_script("for (var let of values) { total = total + let; }")
        .expect("source should parse");
    let [Stmt::ForOf { left, .. }] = script.body.as_slice() else {
        panic!("expected one for-of statement");
    };
    assert!(matches!(
        left,
        ForInLeft::VarDecl {
            binding: BindingPattern::Identifier { name, .. },
            kind: VarKind::Var,
            ..
        } if name == "let"
    ));
    assert!(parse_script("for (let let of values) {}").is_err());
    assert!(parse_script("for (const let of values) {}").is_err());
}

#[test]
fn parses_switch_statement() {
    let script =
        parse_script("switch (x) { case 1: x += 1; break; default: x = 0; case 2: x += 2; }")
            .expect("source should parse");
    let [Stmt::Switch { cases, .. }] = script.body.as_slice() else {
        panic!("expected one switch statement");
    };
    assert_eq!(cases.len(), 3);
    assert!(cases[0].test.is_some());
    assert!(cases[1].test.is_none());
    assert!(cases[2].test.is_some());
    assert_eq!(cases[0].consequent.len(), 2);
}

#[test]
fn parses_break_and_continue_statements() {
    let script = parse_script("outer: while (true) { continue outer; break outer; }")
        .expect("source should parse");
    let [Stmt::Labelled { label, body, .. }] = script.body.as_slice() else {
        panic!("expected one labelled statement");
    };
    assert_eq!(label, "outer");
    let Stmt::While { body, .. } = body.as_ref() else {
        panic!("expected labelled while statement");
    };
    let Stmt::Block { body, .. } = body.as_ref() else {
        panic!("expected block body");
    };
    assert!(matches!(
        body.as_slice(),
        [
            Stmt::Continue {
                label: Some(continue_label),
                ..
            },
            Stmt::Break {
                label: Some(break_label),
                ..
            }
        ] if continue_label == "outer" && break_label == "outer"
    ));
}

#[test]
fn rejects_continue_to_non_iteration_label() {
    assert!(parse_script("label: { for (;;) { continue label; } }").is_err());
}

#[test]
fn parses_throw_statement_with_argument() {
    let script = parse_script("if (false) { throw 'fail'; }").expect("source should parse");
    let [Stmt::If { consequent, .. }] = script.body.as_slice() else {
        panic!("expected one if statement");
    };
    let Stmt::Block { body, .. } = consequent.as_ref() else {
        panic!("expected block consequent");
    };
    assert!(matches!(
        body.as_slice(),
        [Stmt::Throw {
            argument: Some(_),
            ..
        }]
    ));
}

#[test]
fn parses_try_catch_finally_statement() {
    let script = parse_script("try { throw 'x'; } catch (error) { error; } finally { debugger; }")
        .expect("source should parse");
    let [
        Stmt::Try {
            block,
            handler,
            finalizer,
            ..
        },
    ] = script.body.as_slice()
    else {
        panic!("expected one try statement");
    };
    assert!(matches!(block.as_slice(), [Stmt::Throw { .. }]));
    let handler = handler.as_ref().expect("expected catch clause");
    assert!(matches!(
        &handler.param,
        Some(BindingPattern::Identifier { name, .. }) if name == "error"
    ));
    assert_eq!(handler.body.len(), 1);
    assert!(matches!(
        finalizer.as_deref(),
        Some([Stmt::Debugger { .. }])
    ));

    let script = parse_script("try { 1; } finally { 2; }").expect("source should parse");
    assert!(matches!(
        script.body.as_slice(),
        [Stmt::Try {
            handler: None,
            finalizer: Some(_),
            ..
        }]
    ));

    let script = parse_script("try { throw {}; } catch ({ error, value, }) { value; }")
        .expect("source should parse");
    let [
        Stmt::Try {
            handler: Some(handler),
            ..
        },
    ] = script.body.as_slice()
    else {
        panic!("expected try statement with catch clause");
    };
    let Some(BindingPattern::Object { properties, .. }) = &handler.param else {
        panic!("expected object catch binding pattern");
    };
    assert_eq!(
        properties
            .iter()
            .map(|property| property.key.as_literal().expect("literal key"))
            .collect::<Vec<_>>(),
        ["error", "value"]
    );
}

#[test]
fn parses_debugger_statement() {
    let script = parse_script("debugger; 1;").expect("source should parse");
    assert!(matches!(
        script.body.as_slice(),
        [Stmt::Debugger { .. }, Stmt::Expr(_)]
    ));
}

#[test]
fn parses_destructuring_loop_heads_and_catch_patterns() {
    let script = parse_script("for (const [a, , b = 1] of pairs) a;").expect("source should parse");
    let [Stmt::ForOf { left, .. }] = script.body.as_slice() else {
        panic!("expected one for-of statement");
    };
    assert!(matches!(
        left,
        ForInLeft::VarDecl {
            kind: VarKind::Const,
            binding: BindingPattern::Array { .. },
            init: None,
            ..
        }
    ));

    let script = parse_script("for (var {key, value} in store) key;").expect("source should parse");
    let [Stmt::ForIn { left, .. }] = script.body.as_slice() else {
        panic!("expected one for-in statement");
    };
    assert!(matches!(
        left,
        ForInLeft::VarDecl {
            kind: VarKind::Var,
            binding: BindingPattern::Object { .. },
            ..
        }
    ));

    let script = parse_script("for ([a, b] of pairs) a;").expect("source should parse");
    let [Stmt::ForOf { left, .. }] = script.body.as_slice() else {
        panic!("expected one for-of statement");
    };
    assert!(matches!(
        left,
        ForInLeft::Target(qjs_ast::AssignmentTarget::ArrayPattern { .. })
    ));

    let script = parse_script("for ({a} in keys) a;").expect("source should parse");
    let [Stmt::ForIn { left, .. }] = script.body.as_slice() else {
        panic!("expected one for-in statement");
    };
    assert!(matches!(
        left,
        ForInLeft::Target(qjs_ast::AssignmentTarget::ObjectPattern { .. })
    ));

    let script = parse_script("try {} catch ([a, {b = 1}]) { a; }").expect("source should parse");
    let [Stmt::Try { handler, .. }] = script.body.as_slice() else {
        panic!("expected one try statement");
    };
    let handler = handler.as_ref().expect("expected catch clause");
    assert!(matches!(handler.param, Some(BindingPattern::Array { .. })));

    // Pattern heads without `of`/`in` still parse as ordinary for loops.
    parse_script("for (let [a] = [1]; a < 2; a += 1) a;").expect("source should parse");
}

#[test]
fn parses_with_statement() {
    let script = parse_script("with (obj) body;").expect("source should parse");
    let [Stmt::With { object, body, .. }] = script.body.as_slice() else {
        panic!("expected one with statement");
    };
    assert!(matches!(object, Expr::Identifier { name, .. } if name == "obj"));
    assert!(matches!(body.as_ref(), Stmt::Expr(Expr::Identifier { .. })));

    // `with` remains usable as a property name (it is a reserved word only as a
    // statement keyword).
    parse_script("array.with(0, 9);").expect("source should parse");

    // `with` is a SyntaxError in strict-mode code.
    assert!(parse_script("'use strict'; with (obj) {}").is_err());
}

#[test]
fn rejects_disallowed_declarations_in_statement_body() {
    let decls = [
        "class C {}",
        "let x = 1;",
        "const x = 1;",
        "function* g() {}",
        "async function f() {}",
        "async function* g() {}",
    ];

    // Iteration statements reject ALL declarations including plain functions
    let iteration_contexts = [
        ("while (false) CLASS", "while"),
        ("do CLASS while (false);", "do-while"),
        ("for (;;) CLASS", "for"),
        ("for (var x in {}) CLASS", "for-in"),
        ("for (var x of []) CLASS", "for-of"),
    ];
    for (template, ctx) in &iteration_contexts {
        for decl in &decls {
            let code = template.replace("CLASS", decl);
            assert!(parse_script(&code).is_err(), "should reject {ctx}: {code}");
        }
        let fn_code = template.replace("CLASS", "function f() {}");
        assert!(
            parse_script(&fn_code).is_err(),
            "should reject sloppy function in iteration {ctx}: {fn_code}"
        );
    }

    // If/else allows sloppy-mode plain function declarations (Annex B)
    let if_contexts = [
        ("if (true) CLASS", "if"),
        ("if (true) ; else CLASS", "else"),
    ];
    for (template, ctx) in &if_contexts {
        for decl in &decls {
            let code = template.replace("CLASS", decl);
            assert!(parse_script(&code).is_err(), "should reject {ctx}: {code}");
        }
        let fn_code = template.replace("CLASS", "function f() {}");
        assert!(
            parse_script(&fn_code).is_ok(),
            "should allow sloppy function in {ctx}: {fn_code}"
        );
    }
}

#[test]
fn rejects_label_nested_inside_same_label() {
    // A label may not appear inside a statement carrying the same label,
    // whether directly or more deeply nested.
    assert!(parse_script("foo: foo: 0;").is_err());
    assert!(parse_script("foo: { bar: { foo: 0; } }").is_err());
    // Sibling reuse and distinct nested labels remain valid.
    assert!(parse_script("foo: 0; foo: 1;").is_ok());
    assert!(parse_script("a: while (true) { b: while (true) { break a; } }").is_ok());
}

#[test]
fn rejects_disallowed_declarations_in_labelled_body() {
    assert!(parse_script("label: class C {}").is_err());
    assert!(parse_script("label: let x = 1;").is_err());
    assert!(parse_script("label: function* g() {}").is_err());
    assert!(parse_script("label: async function f() {}").is_err());
    assert!(parse_script("label: function f() {}").is_ok());
    assert!(parse_script("\"use strict\"; label: function f() {}").is_err());
}

#[test]
fn rejects_duplicate_lexical_declarations_in_switch() {
    assert!(parse_script("switch(0){case 1: class f {} default: class f{}}").is_err());
    assert!(parse_script("switch(0){case 1: let f; default: let f;}").is_err());
    assert!(parse_script("switch(0){case 1: const f=1; default: const f=2;}").is_err());
    assert!(parse_script("switch(0){case 1: function* f(){} default: function* f(){}}").is_err());
    assert!(parse_script("switch(0){case 1: var f; default: var f;}").is_ok());
    assert!(parse_script("switch(0){case 1: function f(){} default: function f(){}}").is_ok());
    assert!(
        parse_script("\"use strict\"; switch(0){case 1: function f(){} default: function f(){}}")
            .is_err()
    );
}

#[test]
fn rejects_catch_parameter_early_errors() {
    // Duplicate bound names, and collisions with the block's lexical
    // declarations, are Syntax Errors; a `var` redeclaration stays legal.
    assert!(parse_script("try {} catch ([x, x]) {}").is_err());
    assert!(parse_script("try {} catch (x) { let x; }").is_err());
    assert!(parse_script("try {} catch (e) { function e() {} }").is_err());
    assert!(parse_script("try {} catch (x) { var x; }").is_ok());
    assert!(parse_script("try {} catch (x) { let y; }").is_ok());
}

#[test]
fn rejects_await_label_in_class_static_block() {
    // A class static block is an [+Await] context, so `await` is a reserved
    // LabelIdentifier there.
    assert!(parse_script("class C { static { await: 0; } }").is_err());
    // `await` remains a valid label in ordinary (non-async) code.
    assert!(parse_script("await: 0;").is_ok());
}
