//! Module record, linking, evaluation, and namespace tests (T012 S2).

use crate::{EvalErrorKind, MapResolver, Value, eval_module, eval_module_with_prelude};

/// Evaluates the module graph rooted at `"main"`, with extra `(key, source)`
/// modules registered in an in-memory resolver. Returns the root namespace.
fn run(main: &str, deps: &[(&str, &str)]) -> Result<Value, crate::EvalError> {
    let mut resolver = MapResolver::new();
    for (key, source) in deps {
        resolver = resolver.with(key, source);
    }
    eval_module(main, "main", &mut resolver)
}

/// Reads a named export off a namespace object value.
fn export(namespace: &Value, name: &str) -> Value {
    match namespace {
        Value::Object(object) => object
            .own_property(name)
            .map(|property| property.value)
            .unwrap_or(Value::Undefined),
        _ => panic!("expected a namespace object"),
    }
}

#[test]
fn default_and_named_roundtrip() {
    let namespace = run(
        "import def, { named } from \"dep\";\n\
         export const fromDefault = def;\n\
         export const fromNamed = named;",
        &[("dep", "export default 7;\nexport const named = 9;")],
    )
    .expect("graph evaluates");
    assert_eq!(export(&namespace, "fromDefault"), Value::Number(7.0));
    assert_eq!(export(&namespace, "fromNamed"), Value::Number(9.0));
}

#[test]
fn live_binding_through_exported_function() {
    // The importer calls an exported function that reads the exporter's own
    // live top-level binding, observing the post-mutation value.
    let namespace = run(
        "import { bump, current } from \"counter\";\n\
         bump();\n\
         bump();\n\
         export const value = current();",
        &[(
            "counter",
            "var n = 0;\n\
             export function bump() { n = n + 1; }\n\
             export function current() { return n; }",
        )],
    )
    .expect("graph evaluates");
    assert_eq!(export(&namespace, "value"), Value::Number(2.0));
}

#[test]
fn mutual_cycle_with_hoisted_functions() {
    // Two modules import each other's hoisted functions; evaluation of one
    // calls into the other, which is usable because function declarations are
    // available throughout the module.
    let namespace = run(
        "import { ping } from \"a\";\nexport const result = ping(3);",
        &[
            (
                "a",
                "import { pong } from \"b\";\n\
                 export function ping(n) { return n <= 0 ? 0 : 1 + pong(n - 1); }",
            ),
            (
                "b",
                "import { ping } from \"a\";\n\
                 export function pong(n) { return n <= 0 ? 0 : 1 + ping(n - 1); }",
            ),
        ],
    )
    .expect("cyclic graph evaluates");
    assert_eq!(export(&namespace, "result"), Value::Number(3.0));
}

#[test]
fn star_export_aggregation() {
    let namespace = run(
        "import * as ns from \"agg\";\n\
         export const a = ns.a;\n\
         export const b = ns.b;",
        &[
            ("agg", "export * from \"one\";\nexport * from \"two\";"),
            ("one", "export const a = 1;"),
            ("two", "export const b = 2;"),
        ],
    )
    .expect("star aggregation evaluates");
    assert_eq!(export(&namespace, "a"), Value::Number(1.0));
    assert_eq!(export(&namespace, "b"), Value::Number(2.0));
}

#[test]
fn ambiguous_star_export_is_syntax_error() {
    let error = run(
        "import { x } from \"agg\";\nexport const v = x;",
        &[
            ("agg", "export * from \"one\";\nexport * from \"two\";"),
            ("one", "export const x = 1;"),
            ("two", "export const x = 2;"),
        ],
    )
    .expect_err("ambiguous star export rejected");
    assert_eq!(error.kind, EvalErrorKind::Early);
    assert!(error.message.contains("ambiguous"), "{}", error.message);
}

#[test]
fn namespace_object_shape() {
    let namespace = run(
        "export const b = 2;\nexport const a = 1;\nexport default 3;",
        &[],
    )
    .expect("module evaluates");
    let object = match &namespace {
        Value::Object(object) => object.clone(),
        _ => panic!("expected namespace"),
    };
    // Own names are sorted; default is included.
    assert_eq!(object.own_property_names(), vec!["a", "b", "default"]);
    assert_eq!(object.to_string_tag().as_deref(), Some("Module"));
    assert!(!object.is_extensible());
}

#[test]
fn unresolvable_import_is_syntax_error() {
    let error = run("import { x } from \"missing\";\nexport const v = x;", &[])
        .expect_err("missing module rejected");
    assert_eq!(error.kind, EvalErrorKind::Early);
}

#[test]
fn missing_named_export_is_syntax_error() {
    let error = run(
        "import { nope } from \"dep\";\nexport const v = nope;",
        &[("dep", "export const yes = 1;")],
    )
    .expect_err("missing named export rejected");
    assert_eq!(error.kind, EvalErrorKind::Early);
    assert!(error.message.contains("no export"), "{}", error.message);
}

#[test]
fn module_vars_do_not_leak_to_global_this() {
    // A module-scoped `var` must not become a property of the host globalThis,
    // and a subsequent script must not see it.
    run("var leaked = 123;\nexport const v = leaked;", &[]).expect("module evaluates");
    let result = crate::eval("typeof leaked;").expect("script evaluates");
    assert_eq!(result, Value::String("undefined".to_owned()));
}

#[test]
fn prelude_script_bindings_are_visible_to_module() {
    // A prelude script (mirroring Test262 harness includes) installs a global
    // helper that the module body then calls; its value flows into an export.
    let mut resolver = MapResolver::new();
    let namespace = eval_module_with_prelude(
        Some("function helper() { return 11; }"),
        "export const v = helper();",
        "main",
        &mut resolver,
    )
    .expect("module with prelude evaluates");
    assert_eq!(export(&namespace, "v"), Value::Number(11.0));
}

#[test]
fn prelude_throw_is_a_runtime_error() {
    let mut resolver = MapResolver::new();
    let error = eval_module_with_prelude(
        Some("throw new Error('boom');"),
        "export const v = 1;",
        "main",
        &mut resolver,
    )
    .expect_err("prelude failure surfaces");
    assert_eq!(error.kind, EvalErrorKind::Runtime);
}

#[test]
fn module_body_is_strict_mode() {
    // Assigning to an undeclared name is a ReferenceError in strict mode.
    let error =
        run("undeclared = 1;\nexport const v = 1;", &[]).expect_err("strict assignment rejected");
    assert_eq!(error.kind, EvalErrorKind::Runtime);
}
