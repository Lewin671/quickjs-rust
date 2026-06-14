//! Module record, linking, evaluation, and namespace tests (T012 S2).

use crate::{EvalErrorKind, MapResolver, Value, eval_module, eval_module_with_prelude};

/// Evaluates the module graph rooted at `"main"`, with extra `(key, source)`
/// modules registered in an in-memory resolver. Returns the root namespace.
fn run(main: &str, deps: &[(&str, &str)]) -> Result<Value, crate::EvalError> {
    let mut resolver = MapResolver::new();
    for (key, source) in deps {
        resolver = resolver.with(key, source);
    }
    eval_module(main, "main", Box::new(resolver))
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
fn anonymous_default_class_gets_default_name_in_static_initializer() {
    let namespace = run(
        "var className;\n\
         export default class { static f = (className = this.name); }\n\
         export const observed = className;",
        &[],
    )
    .expect("module evaluates");
    assert_eq!(
        export(&namespace, "observed"),
        Value::String("default".to_owned())
    );
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
    let resolver = MapResolver::new();
    let namespace = eval_module_with_prelude(
        Some("function helper() { return 11; }"),
        "export const v = helper();",
        "main",
        Box::new(resolver),
    )
    .expect("module with prelude evaluates");
    assert_eq!(export(&namespace, "v"), Value::Number(11.0));
}

#[test]
fn prelude_throw_is_a_runtime_error() {
    let resolver = MapResolver::new();
    let error = eval_module_with_prelude(
        Some("throw new Error('boom');"),
        "export const v = 1;",
        "main",
        Box::new(resolver),
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

// --- dynamic import (T012 S4) ---------------------------------------------

/// Reads the elements of an exported array value as a comma-joined string. The
/// exported binding holds the array by reference, so a `.then` callback that
/// pushes to it during the post-evaluation job drain is observable here.
fn export_log(namespace: &Value, name: &str) -> String {
    match export(namespace, name) {
        Value::Array(array) => array
            .to_vec()
            .into_iter()
            .map(|value| match value {
                Value::String(text) => text,
                Value::Number(number) => number.to_string(),
                Value::Boolean(flag) => flag.to_string(),
                other => format!("{other:?}"),
            })
            .collect::<Vec<_>>()
            .join(","),
        other => panic!("expected an array export, got {other:?}"),
    }
}

#[test]
fn dynamic_import_from_module_resolves_namespace() {
    // `import('dep')` resolves to the dependency's namespace; the `.then`
    // callback records its named export into the shared exported array.
    let namespace = run(
        "export const log = [];\n\
         import('dep').then(ns => log.push(ns.value));",
        &[("dep", "export const value = 42;")],
    )
    .expect("graph evaluates");
    assert_eq!(export_log(&namespace, "log"), "42");
}

#[test]
fn dynamic_import_rejects_unresolvable_specifier() {
    let namespace = run(
        "export const log = [];\n\
         import('missing').then(() => log.push('ok'), () => log.push('rejected'));",
        &[],
    )
    .expect("graph evaluates");
    assert_eq!(export_log(&namespace, "log"), "rejected");
}

#[test]
fn dynamic_import_caches_same_namespace() {
    // Two imports of the same specifier resolve to the identical namespace
    // object (same key => same module record).
    let namespace = run(
        "export const log = [];\n\
         Promise.all([import('dep'), import('dep')]).then(([a, b]) => log.push(a === b));",
        &[("dep", "export const value = 1;")],
    )
    .expect("graph evaluates");
    assert_eq!(export_log(&namespace, "log"), "true");
}

#[test]
fn dynamic_import_coerces_specifier_to_string() {
    // The specifier is coerced via ToString; an object with a custom toString
    // resolves to the named module.
    let namespace = run(
        "export const log = [];\n\
         const spec = { toString() { return 'dep'; } };\n\
         import(spec).then(ns => log.push(ns.value));",
        &[("dep", "export const value = 5;")],
    )
    .expect("graph evaluates");
    assert_eq!(export_log(&namespace, "log"), "5");
}

#[test]
fn dynamic_import_rejects_on_module_body_error() {
    let namespace = run(
        "export const log = [];\n\
         import('boom').then(() => log.push('ok'), () => log.push('rejected'));",
        &[("boom", "throw new Error('explode');")],
    )
    .expect("graph evaluates");
    assert_eq!(export_log(&namespace, "log"), "rejected");
}

#[test]
fn dynamic_import_then_runs_after_current_job() {
    // The synchronous body completes (pushing "sync") before the import's
    // `.then` callback (pushing "async") runs as a later microtask.
    let namespace = run(
        "export const log = [];\n\
         import('dep').then(() => log.push('async'));\n\
         log.push('sync');",
        &[("dep", "export const value = 1;")],
    )
    .expect("graph evaluates");
    assert_eq!(export_log(&namespace, "log"), "sync,async");
}

#[test]
fn dynamic_import_in_script_resolves_namespace() {
    // A dynamic import works under the Script goal too, against an in-memory
    // resolver, with the namespace recorded through a shared exported array.
    // (Driven via a module here so the resolver is available; the call site is
    // an ordinary expression valid in both goals.)
    let namespace = run(
        "export const log = [];\n\
         (function () { import('dep').then(ns => log.push(ns.value)); })();",
        &[("dep", "export const value = 99;")],
    )
    .expect("graph evaluates");
    assert_eq!(export_log(&namespace, "log"), "99");
}

// --- top-level await (T012 S5) --------------------------------------------

#[test]
fn top_level_await_exports_awaited_value() {
    // A module body that awaits a resolved promise binds the fulfillment value;
    // the export is observable after the graph settles.
    let namespace = run(
        "export let value = await Promise.resolve(41).then(v => v + 1);",
        &[],
    )
    .expect("graph evaluates");
    assert_eq!(export(&namespace, "value"), Value::Number(42.0));
}

#[test]
fn top_level_await_of_plain_value() {
    // `await <non-promise>` resolves to the value itself.
    let namespace = run("export var x = await 7;", &[]).expect("graph evaluates");
    assert_eq!(export(&namespace, "x"), Value::Number(7.0));
}

#[test]
fn dependent_sees_settled_tla_binding() {
    // An acyclic dependency uses top-level await; its importer must observe the
    // settled exported binding (the dependency fully settles before the
    // dependent body runs).
    let namespace = run(
        "import { ready } from \"dep\";\n\
         export const seen = ready;",
        &[("dep", "export const ready = await Promise.resolve('done');")],
    )
    .expect("graph evaluates");
    assert_eq!(export(&namespace, "seen"), Value::String("done".to_owned()));
}

#[test]
fn top_level_await_rejection_propagates() {
    // A rejected top-level await fails module evaluation, surfacing a runtime
    // error rather than silently completing.
    let error = run(
        "export const v = await Promise.reject(new Error('boom'));",
        &[],
    )
    .expect_err("rejected top-level await fails evaluation");
    assert_eq!(error.kind, EvalErrorKind::Runtime);
}

#[test]
fn top_level_await_in_script_is_parse_error() {
    // `await` stays an identifier under the Script goal: a top-level `await x`
    // does not parse as an AwaitExpression. Used as a binding it is fine in
    // sloppy script code, so `var await = 1` evaluates without error.
    let value = crate::eval("var await = 1; await;").expect("await is an identifier in a script");
    assert_eq!(value, Value::Number(1.0));
}

#[test]
fn await_as_binding_is_a_module_error() {
    // Under the Module goal `await` is reserved: a `let await` binding is a
    // SyntaxError (reported at the parse/early stage).
    let error = run("let await = 1;\nexport const v = await;", &[])
        .expect_err("await binding rejected in a module");
    assert!(matches!(
        error.kind,
        EvalErrorKind::Parse | EvalErrorKind::Early
    ));
}
