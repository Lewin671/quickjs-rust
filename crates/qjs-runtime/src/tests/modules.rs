//! Runtime behavior for module items (T012 S1).
//!
//! The parser accepts `import`/`export` under the Module goal, but the runtime
//! does not yet link or evaluate modules, so compiling a module body must fail
//! with a structured "modules are not yet supported" error rather than panic.

use qjs_parser::parse_module;

use crate::bytecode::compile_script_classified;

fn compile_module_error(source: &str) -> String {
    let script = parse_module(source).expect("module source should parse");
    let error =
        compile_script_classified(&script).expect_err("module compilation is not yet supported");
    error.error.message
}

#[test]
fn import_declaration_is_unsupported() {
    assert_eq!(
        compile_module_error("import x from \"mod\";"),
        "modules are not yet supported"
    );
}

#[test]
fn export_declaration_is_unsupported() {
    assert_eq!(
        compile_module_error("export const x = 1;"),
        "modules are not yet supported"
    );
}

#[test]
fn export_default_is_unsupported() {
    assert_eq!(
        compile_module_error("export default 1;"),
        "modules are not yet supported"
    );
}
