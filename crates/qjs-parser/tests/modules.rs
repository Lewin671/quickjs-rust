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
