//! Integration tests for module-goal parsing (`parse_module`).
//!
//! These exercise the public `parse_module` entry against the `import`/`export`
//! declaration forms and confirm that script-mode parsing is unaffected.

use qjs_ast::{DefaultExport, ExportDecl, ImportSpecifier, ModuleDecl, ModuleExportName, Stmt};
use qjs_parser::{parse_module, parse_script};

fn sole_module_decl(source: &str) -> ModuleDecl {
    let script = parse_module(source).expect("module source should parse");
    let [Stmt::ModuleDecl(decl)] = script.body.as_slice() else {
        panic!(
            "expected a single module declaration, got {:?}",
            script.body
        );
    };
    decl.clone()
}

fn sole_import(source: &str) -> (Vec<ImportSpecifier>, String) {
    match sole_module_decl(source) {
        ModuleDecl::Import(decl) => (decl.specifiers, decl.source),
        other => panic!("expected an import declaration, got {other:?}"),
    }
}

fn sole_export(source: &str) -> ExportDecl {
    match sole_module_decl(source) {
        ModuleDecl::Export(decl) => decl,
        other => panic!("expected an export declaration, got {other:?}"),
    }
}

#[test]
fn parses_side_effect_import() {
    let (specifiers, source) = sole_import("import \"mod\";");
    assert!(specifiers.is_empty());
    assert_eq!(source, "mod");
}

#[test]
fn parses_default_import() {
    let (specifiers, source) = sole_import("import def from \"mod\";");
    assert_eq!(source, "mod");
    let [ImportSpecifier::Default { local, .. }] = specifiers.as_slice() else {
        panic!("expected a single default specifier");
    };
    assert_eq!(local, "def");
}

#[test]
fn parses_namespace_import() {
    let (specifiers, _) = sole_import("import * as ns from \"mod\";");
    let [ImportSpecifier::Namespace { local, .. }] = specifiers.as_slice() else {
        panic!("expected a single namespace specifier");
    };
    assert_eq!(local, "ns");
}

#[test]
fn parses_named_imports_with_alias() {
    let (specifiers, _) = sole_import("import { a, b as c } from \"mod\";");
    let [first, second] = specifiers.as_slice() else {
        panic!("expected two named specifiers");
    };
    let ImportSpecifier::Named {
        imported, local, ..
    } = first
    else {
        panic!("expected a named specifier");
    };
    assert_eq!(imported.as_str(), "a");
    assert_eq!(local, "a");
    let ImportSpecifier::Named {
        imported, local, ..
    } = second
    else {
        panic!("expected a named specifier");
    };
    assert_eq!(imported.as_str(), "b");
    assert_eq!(local, "c");
}

#[test]
fn parses_default_and_named_combination() {
    let (specifiers, _) = sole_import("import def, { a as b } from \"mod\";");
    assert!(matches!(specifiers[0], ImportSpecifier::Default { .. }));
    assert!(matches!(specifiers[1], ImportSpecifier::Named { .. }));
}

#[test]
fn parses_default_and_namespace_combination() {
    let (specifiers, _) = sole_import("import def, * as ns from \"mod\";");
    assert!(matches!(specifiers[0], ImportSpecifier::Default { .. }));
    assert!(matches!(specifiers[1], ImportSpecifier::Namespace { .. }));
}

#[test]
fn parses_named_export_clause() {
    let ExportDecl::Named {
        specifiers, source, ..
    } = sole_export("export { a, b as c };")
    else {
        panic!("expected a named export");
    };
    assert!(source.is_none());
    assert_eq!(specifiers[0].local.as_str(), "a");
    assert_eq!(specifiers[0].exported.as_str(), "a");
    assert_eq!(specifiers[1].local.as_str(), "b");
    assert_eq!(specifiers[1].exported.as_str(), "c");
}

#[test]
fn parses_named_reexport() {
    let ExportDecl::Named {
        specifiers, source, ..
    } = sole_export("export { x } from \"mod\";")
    else {
        panic!("expected a named re-export");
    };
    assert_eq!(source.as_deref(), Some("mod"));
    assert_eq!(specifiers[0].exported.as_str(), "x");
}

#[test]
fn parses_star_reexport() {
    let ExportDecl::All {
        exported, source, ..
    } = sole_export("export * from \"mod\";")
    else {
        panic!("expected a star re-export");
    };
    assert!(exported.is_none());
    assert_eq!(source, "mod");
}

#[test]
fn parses_namespace_reexport() {
    let ExportDecl::All {
        exported, source, ..
    } = sole_export("export * as ns from \"mod\";")
    else {
        panic!("expected a namespace re-export");
    };
    assert_eq!(exported.as_ref().map(ModuleExportName::as_str), Some("ns"));
    assert_eq!(source, "mod");
}

#[test]
fn parses_export_default_expression() {
    let ExportDecl::Default { declaration, .. } = sole_export("export default 1 + 2;") else {
        panic!("expected a default export");
    };
    assert!(matches!(declaration, DefaultExport::Expression(_)));
}

#[test]
fn parses_export_default_function() {
    let ExportDecl::Default { declaration, .. } = sole_export("export default function f() {}")
    else {
        panic!("expected a default export");
    };
    let DefaultExport::Declaration(stmt) = declaration else {
        panic!("expected a function declaration default");
    };
    assert!(matches!(*stmt, Stmt::FunctionDecl { .. }));
}

#[test]
fn parses_export_default_class() {
    let ExportDecl::Default { declaration, .. } = sole_export("export default class C {}") else {
        panic!("expected a default export");
    };
    let DefaultExport::Declaration(stmt) = declaration else {
        panic!("expected a class declaration default");
    };
    assert!(matches!(*stmt, Stmt::ClassDecl { .. }));
}

#[test]
fn parses_export_declaration() {
    let ExportDecl::Declaration { declaration, .. } = sole_export("export const x = 1;") else {
        panic!("expected an export declaration");
    };
    assert!(matches!(*declaration, Stmt::VarDecl { .. }));
}

#[test]
fn parses_export_function_declaration() {
    let ExportDecl::Declaration { declaration, .. } = sole_export("export function f() {}") else {
        panic!("expected an export declaration");
    };
    assert!(matches!(*declaration, Stmt::FunctionDecl { .. }));
}

#[test]
fn module_allows_ordinary_statements() {
    let script = parse_module("const x = 1; export { x };").expect("should parse");
    assert!(matches!(script.body[0], Stmt::VarDecl { .. }));
    assert!(matches!(script.body[1], Stmt::ModuleDecl(_)));
}

#[test]
fn module_spans_cover_declaration() {
    let decl = sole_module_decl("import x from \"mod\";");
    assert_eq!(decl.span().start, 0);
    assert!(decl.span().end > 0);
}

#[test]
fn import_with_string_export_name_requires_alias() {
    let error = parse_module("import { \"a-b\" } from \"mod\";")
        .expect_err("a string import name needs an alias");
    assert!(error.message.contains("as"));
}

#[test]
fn import_with_string_export_name_and_alias_parses() {
    let (specifiers, _) = sole_import("import { \"a-b\" as c } from \"mod\";");
    let [
        ImportSpecifier::Named {
            imported, local, ..
        },
    ] = specifiers.as_slice()
    else {
        panic!("expected a named specifier");
    };
    assert!(matches!(imported, ModuleExportName::String(name) if name == "a-b"));
    assert_eq!(local, "c");
}

// --- script-mode regression guards -------------------------------------------

#[test]
fn script_mode_treats_import_as_identifier() {
    // `import` is an ordinary identifier in script source; this is a comma
    // expression statement, not a module item.
    let script = parse_script("import, x;").expect("script source should parse");
    assert!(matches!(script.body[0], Stmt::Expr(_)));
}

#[test]
fn script_mode_does_not_produce_module_items() {
    let script = parse_script("var export_name = 1;").expect("script source should parse");
    assert!(
        script
            .body
            .iter()
            .all(|stmt| !matches!(stmt, Stmt::ModuleDecl(_)))
    );
}

#[test]
fn module_export_after_var_uses_existing_binding() {
    // Smoke test that a multi-item module body parses end to end.
    let script =
        parse_module("import def, { a as b } from \"a\";\nexport { b };\nexport default 42;\n")
            .expect("module should parse");
    assert_eq!(script.body.len(), 3);
}

// --- dynamic import / import.meta (T012 S4) ----------------------------------

/// Extracts the sole expression-statement expression from a *script*.
fn sole_script_expr(source: &str) -> qjs_ast::Expr {
    let script = parse_script(source).expect("script source should parse");
    match script.body.as_slice() {
        [Stmt::Expr(expr)] => expr.clone(),
        other => panic!("expected a single expression statement, got {other:?}"),
    }
}

#[test]
fn parses_dynamic_import_call_in_script() {
    let expr = sole_script_expr("import('./mod.js');");
    let qjs_ast::Expr::ImportCall {
        specifier, options, ..
    } = expr
    else {
        panic!("expected an ImportCall expression, got {expr:?}");
    };
    assert!(matches!(
        *specifier,
        qjs_ast::Expr::Literal(qjs_ast::Literal::String { .. })
    ));
    assert!(options.is_none());
}

#[test]
fn parses_dynamic_import_call_in_module() {
    let script = parse_module("import('./mod.js');").expect("module should parse");
    assert!(matches!(
        script.body.as_slice(),
        [Stmt::Expr(qjs_ast::Expr::ImportCall { .. })]
    ));
}

#[test]
fn parses_dynamic_import_with_options_and_trailing_comma() {
    let expr = sole_script_expr("import('./mod.js', { with: { type: 'json' } },);");
    let qjs_ast::Expr::ImportCall { options, .. } = expr else {
        panic!("expected an ImportCall expression, got {expr:?}");
    };
    assert!(options.is_some());
}

#[test]
fn parses_import_meta() {
    let script = parse_module("import.meta;").expect("module should parse");
    assert!(matches!(
        script.body.as_slice(),
        [Stmt::Expr(qjs_ast::Expr::ImportMeta { .. })]
    ));
}

#[test]
fn rejects_new_import_call() {
    assert!(parse_script("new import('./mod.js');").is_err());
    assert!(parse_module("new import('./mod.js');").is_err());
}

#[test]
fn rejects_empty_import_call() {
    assert!(parse_script("import();").is_err());
}

#[test]
fn rejects_spread_import_argument() {
    assert!(parse_script("import(...['./mod.js']);").is_err());
}

#[test]
fn rejects_three_argument_import_call() {
    assert!(parse_script("import('./mod.js', {}, '');").is_err());
}

// --- top-level await (T012 S5) --------------------------------------------

#[test]
fn parses_top_level_await_expression() {
    // Under the Module goal `await expr` is an AwaitExpression at the top level.
    let script = parse_module("await 1;").expect("top-level await parses in a module");
    assert!(matches!(
        script.body.as_slice(),
        [Stmt::Expr(qjs_ast::Expr::Await { .. })]
    ));
}

#[test]
fn parses_top_level_await_in_block_and_export() {
    // `await` is the keyword form inside module-level blocks and an exported
    // declaration's initializer.
    assert!(parse_module("{ await 1; }").is_ok());
    assert!(parse_module("export const v = await 1;").is_ok());
    assert!(parse_module("if (true) await 1;").is_ok());
}

#[test]
fn await_is_not_a_keyword_in_a_module_nested_function() {
    // An ordinary (non-async) function body resets the await context, so
    // `await` is an ordinary identifier there: it is a legal binding/parameter
    // name inside the nested function even though it is reserved at module top
    // level.
    assert!(parse_module("function f(await) { return await; }").is_ok());
    assert!(parse_module("function f() { var await = 1; return await; }").is_ok());
}

#[test]
fn rejects_await_as_binding_in_a_module() {
    // `await` is a reserved word in module code; using it as a binding name is a
    // SyntaxError.
    assert!(parse_module("let await = 1;").is_err());
    assert!(parse_module("var await = 1;").is_err());
    assert!(parse_module("function await() {}").is_err());
}

#[test]
fn await_remains_an_identifier_in_a_script() {
    // The Script goal is unchanged: `await` is an ordinary identifier, so a
    // top-level `await x` is `await(x)`-style member/identifier usage, not an
    // AwaitExpression, and `var await` is allowed in sloppy script code.
    assert!(parse_script("var await = 1; await;").is_ok());
    assert!(parse_script("let await = 1;").is_ok());
}
