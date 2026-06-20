//! Module record, linking, evaluation, and namespace tests (T012 S2).

use crate::{
    EvalErrorKind, MapResolver, Value, eval_classified_with_resolver, eval_module,
    eval_module_with_prelude,
};

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

fn settled_fulfillment(value: &Value) -> Option<Value> {
    if let Value::Object(object) = value {
        match crate::promise::settled_outcome(object) {
            Some(Ok(value)) => Some(value),
            _ => None,
        }
    } else {
        None
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
fn anonymous_default_function_import_is_callable() {
    let namespace = run(
        "import def from \"dep\";\n\
         export const result = def();\n\
         export const name = def.name;",
        &[("dep", "export default function() { return 23; }")],
    )
    .expect("graph evaluates");
    assert_eq!(export(&namespace, "result"), Value::Number(23.0));
    assert_eq!(
        export(&namespace, "name"),
        Value::String("default".to_owned().into())
    );
}

#[test]
fn named_default_function_binds_local_name() {
    let namespace = run(
        "export default function F() { return 31; }\n\
         F.extra = 11;\n\
         export const local = F();\n\
         export const prop = F.extra;",
        &[],
    )
    .expect("module evaluates");
    assert_eq!(export(&namespace, "local"), Value::Number(31.0));
    assert_eq!(export(&namespace, "prop"), Value::Number(11.0));
    match export(&namespace, "default") {
        Value::Function(function) => assert_eq!(function.name.as_deref(), Some("F")),
        other => panic!("expected default function, got {other:?}"),
    }
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
        Value::String("default".to_owned().into())
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
fn namespace_reexport_exposes_target_namespace() {
    let namespace = run(
        "import { nested } from \"agg\";\n\
         export const value = nested.value;\n\
         export const keys = Object.getOwnPropertyNames(nested).join(',');",
        &[
            ("agg", "export * as nested from \"dep\";"),
            ("dep", "export default 7;\nexport const value = 3;"),
        ],
    )
    .expect("namespace re-export evaluates");
    assert_eq!(export(&namespace, "value"), Value::Number(3.0));
    assert_eq!(
        export(&namespace, "keys"),
        Value::String("default,value".to_owned().into())
    );
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
fn circular_indirect_reexport_is_syntax_error() {
    // `a` re-exports `x` from `b`, `b` re-exports `x` from `a`, neither defines
    // `x`: ResolveExport cycles with no binding, a SyntaxError at link time.
    // The bare `import "a"` does not itself name `x`, so this exercises the
    // module's own indirect-export validation rather than the import check.
    let error = run(
        "import \"a\";\nexport const v = 1;",
        &[
            ("a", "export { x } from \"b\";"),
            ("b", "export { x } from \"a\";"),
        ],
    )
    .expect_err("circular re-export rejected");
    assert_eq!(error.kind, EvalErrorKind::Early);

    // A valid indirect re-export still links and evaluates.
    let namespace = run(
        "import { x } from \"a\";\nexport const v = x;",
        &[
            ("a", "export { x } from \"b\";"),
            ("b", "export const x = 5;"),
        ],
    )
    .expect("valid re-export evaluates");
    assert_eq!(export(&namespace, "v"), Value::Number(5.0));
}

#[test]
fn module_top_level_var_function_collision_is_syntax_error() {
    // At module top level a function declaration is a LexicallyDeclaredName, so
    // it conflicts with a same-named `var` (a Script would accept this via Annex
    // B). The collision is a parse-time SyntaxError.
    let error = run(
        "var smoosh;\nfunction smoosh() {}\nexport const v = 1;",
        &[],
    )
    .expect_err("module var/function collision rejected");
    assert!(error.message.contains("conflicts"), "{}", error.message);

    // Distinct names, and a function declaration alone, stay valid.
    run("var a;\nfunction b() {}\nexport const v = b;", &[]).expect("distinct names evaluate");
}

#[test]
fn namespace_has_own_to_string_tag_property() {
    // A module namespace has an own `@@toStringTag` data property "Module"
    // (writable:false, enumerable:false, configurable:false).
    let namespace = run(
        "import * as ns from \"dep\";\n\
         export const tag = ns[Symbol.toStringTag];\n\
         var d = Object.getOwnPropertyDescriptor(ns, Symbol.toStringTag);\n\
         export const ok = d !== undefined && d.value === 'Module' \
             && d.writable === false && d.enumerable === false && d.configurable === false;",
        &[("dep", "export const x = 1;")],
    )
    .expect("module evaluates");
    assert_eq!(
        export(&namespace, "tag"),
        Value::String("Module".to_owned().into())
    );
    assert_eq!(export(&namespace, "ok"), Value::Boolean(true));
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
fn namespace_export_descriptors_are_writable_but_not_settable() {
    let namespace = run(
        "import * as ns from \"dep\";\n\
         const desc = Object.getOwnPropertyDescriptor(ns, 'value');\n\
         export const attrs = [desc.value, desc.writable, desc.enumerable, desc.configurable].join(':');\n\
         export const setResult = Reflect.set(ns, 'value', 7);\n\
         export const sameDefine = Reflect.defineProperty(ns, 'value', { writable: true, enumerable: true, configurable: false });\n\
         export const changedDefine = Reflect.defineProperty(ns, 'value', { value: 7 });",
        &[("dep", "export const value = 5;")],
    )
    .expect("module evaluates");
    assert_eq!(
        export(&namespace, "attrs"),
        Value::String("5:true:true:false".to_owned().into())
    );
    assert_eq!(export(&namespace, "setResult"), Value::Boolean(false));
    assert_eq!(export(&namespace, "sameDefine"), Value::Boolean(true));
    assert_eq!(export(&namespace, "changedDefine"), Value::Boolean(false));
}

#[test]
fn namespace_self_import_observes_initialized_exports() {
    let namespace = run(
        "import { result } from \"dep\";\n\
         export { result };",
        &[(
            "dep",
            "import * as ns from \"dep\";\n\
             export let value = 23;\n\
             value = 31;\n\
             const desc = Object.getOwnPropertyDescriptor(ns, 'value');\n\
             export const result = ns.value + ':' + desc.value + ':' + desc.writable;",
        )],
    )
    .expect("module evaluates");
    assert_eq!(
        export(&namespace, "result"),
        Value::String("31:31:true".to_owned().into())
    );
}

#[test]
fn namespace_self_imported_var_export_is_initialized_undefined() {
    let namespace = run(
        "import { result } from \"dep\";\n\
         export { result };",
        &[(
            "dep",
            "import * as ns from \"dep\";\n\
         const desc = Object.getOwnPropertyDescriptor(ns, 'value');\n\
         export var value;\n\
         export const result = (desc.value === undefined) + ':' + desc.writable + ':' + Object.getOwnPropertyNames(ns).join(',');",
        )],
    )
    .expect("module evaluates");
    assert_eq!(
        export(&namespace, "result"),
        Value::String("true:true:result,value".to_owned().into())
    );
}

#[test]
fn namespace_self_import_observes_var_default_and_indirect_exports() {
    let namespace = run(
        "import { result } from \"dep\";\n\
         export { result };",
        &[(
            "dep",
            "import * as ns from \"dep\";\n\
             export var local1 = 201;\n\
             var local2 = 207;\n\
             export { local2 as renamed };\n\
             export { local1 as indirect } from \"dep\";\n\
             export default 302;\n\
             export const result = ns.local1 + ':' + ns.renamed + ':' + ns.indirect + ':' + ns.default;",
        )],
    )
    .expect("module evaluates");
    assert_eq!(
        export(&namespace, "result"),
        Value::String("201:207:201:302".to_owned().into())
    );
}

#[test]
fn namespace_self_imported_lexical_export_throws_before_initialization() {
    let error = run(
        "import \"dep\";\n\
         export const result = 1;",
        &[(
            "dep",
            "import * as ns from \"dep\";\n\
             ns.value;\n\
             export let value = 23;",
        )],
    )
    .expect_err("namespace lexical export should be in TDZ");
    assert_eq!(error.kind, EvalErrorKind::Runtime);
    assert!(
        error.message.contains("ReferenceError"),
        "{}",
        error.message
    );
}

#[test]
fn namespace_self_imported_all_lexical_export_forms_throw_before_initialization() {
    for (access, message) in [
        ("ns.local1;", "local export"),
        ("ns.renamed;", "renamed export"),
        ("ns.indirect;", "indirect export"),
        ("ns.default;", "default export"),
    ] {
        let source = format!(
            "import * as ns from \"dep\";\n\
             {access}\n\
             export let local1 = 23;\n\
             let local2 = 45;\n\
             export {{ local2 as renamed }};\n\
             export {{ local1 as indirect }} from \"dep\";\n\
             export default null;"
        );
        let error = run(
            "import \"dep\";\n\
             export const result = 1;",
            &[("dep", source.as_str())],
        )
        .expect_err(message);
        assert_eq!(error.kind, EvalErrorKind::Runtime, "{message}");
        assert!(
            error.message.contains("ReferenceError"),
            "{message}: {}",
            error.message
        );
    }
}

#[test]
fn namespace_self_imported_tdz_is_observed_by_super_receiver_set() {
    let namespace = run(
        "export { result } from \"dep\";",
        &[(
            "dep",
            "import * as ns from \"dep\";\n\
             class A { constructor() { return ns; } }\n\
             class B extends A { constructor() { super(); super.foo = 14; } }\n\
             let caught = false;\n\
             try { new B(); } catch (error) { caught = error instanceof ReferenceError; }\n\
             export const result = caught;\n\
             export let foo = 42;",
        )],
    )
    .expect("module evaluates");
    assert_eq!(export(&namespace, "result"), Value::Boolean(true));
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
    assert_eq!(result, Value::String("undefined".to_owned().into()));
}

#[test]
fn module_var_and_function_bindings_do_not_pollute_module_global_this() {
    let namespace = run(
        "var test262 = 23;\n\
         function read() { return test262; }\n\
         export const value = read();\n\
         export const hasVar = globalThis.hasOwnProperty('test262');\n\
         export const hasFn = globalThis.hasOwnProperty('read');",
        &[],
    )
    .expect("module evaluates");
    assert_eq!(export(&namespace, "value"), Value::Number(23.0));
    assert_eq!(export(&namespace, "hasVar"), Value::Boolean(false));
    assert_eq!(export(&namespace, "hasFn"), Value::Boolean(false));
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

#[test]
fn dependencies_evaluate_in_requested_module_order() {
    let error = run(
        "import \"first\";\n\
         import \"second\";\n\
         throw new Error('main');",
        &[
            ("first", "throw new TypeError('first');"),
            ("second", "throw new RangeError('second');"),
        ],
    )
    .expect_err("first requested module should fail first");
    assert_eq!(error.kind, EvalErrorKind::Runtime);
    assert!(
        error.message.contains("TypeError") && error.message.contains("first"),
        "{}",
        error.message
    );
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
                Value::String(text) => text.to_string(),
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
fn dynamic_import_namespace_set_same_value_still_fails() {
    let namespace = run(
        "export const log = [];\n\
         import('dep').then(ns => {\n\
           log.push(Reflect.set(ns, 'value', 5));\n\
           try {\n\
             ns.value = 5;\n\
             log.push('assigned');\n\
           } catch (error) {\n\
             log.push(error instanceof TypeError);\n\
           }\n\
         });",
        &[("dep", "export const value = 5;")],
    )
    .expect("graph evaluates");
    assert_eq!(export_log(&namespace, "log"), "false,true");
}

#[test]
fn dynamic_import_namespace_tracks_self_export_updates() {
    let namespace = run(
        "import { log } from 'dep';\n\
         export { log };",
        &[(
            "dep",
            "let x = 0;\n\
             export { x, x as y };\n\
             export const log = [];\n\
             async function main() {\n\
               const imported = await import('dep');\n\
               log.push(imported.x);\n\
               log.push(imported.y);\n\
               x = 1;\n\
               log.push(imported.x);\n\
               log.push(imported.y);\n\
             }\n\
             main();",
        )],
    )
    .expect("graph evaluates");
    assert_eq!(export_log(&namespace, "log"), "0,0,1,1");
}

#[test]
fn dynamic_import_namespace_tracks_updates_after_nested_import() {
    let namespace = run(
        "export const log = [];\n\
         import('dep').then(first => {\n\
           log.push(first.x);\n\
           return first.default().then(other => {\n\
             log.push(first.x);\n\
             log.push(other.default);\n\
           });\n\
         });",
        &[
            (
                "dep",
                "Function('return this;')().global = Function('return this;')();\n\
                 Function('return this;')().test262Update = name => x = name;\n\
                 export default function() { return import('other'); }\n\
                 export var x = 'first';",
            ),
            (
                "other",
                "global.test262Update('other');\n\
                 export default 42;",
            ),
        ],
    )
    .expect("graph evaluates");
    assert_eq!(export_log(&namespace, "log"), "first,other,42");
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
fn dynamic_import_rejects_errored_module_again() {
    let namespace = run(
        "export const log = [];\n\
         async function main() {\n\
           try { await import('boom'); } catch { log.push('first'); }\n\
           try { await import('boom'); } catch { log.push('second'); }\n\
         }\n\
         main();",
        &[("boom", "throw new Error('explode');")],
    )
    .expect("graph evaluates");
    assert_eq!(export_log(&namespace, "log"), "first,second");
}

#[test]
fn for_await_over_dynamic_imports_observes_values_then_rejection() {
    let namespace = run(
        "export const log = [];\n\
         async function main() {\n\
           try {\n\
             for await (const ns of [import('a'), import('b'), import('boom')]) {\n\
               log.push(ns.x);\n\
             }\n\
           } catch (error) {\n\
             log.push(error);\n\
           }\n\
         }\n\
         main();",
        &[
            ("a", "export var x = 42;"),
            ("b", "export var x = 39;"),
            ("boom", "throw 'foo';"),
        ],
    )
    .expect("graph evaluates");
    assert_eq!(export_log(&namespace, "log"), "42,39,foo");
}

#[test]
fn async_generator_yielding_dynamic_imports_rejects_queued_next() {
    let namespace = run(
        "export const log = [];\n\
         async function* gen() {\n\
           yield import('a');\n\
           yield import('b');\n\
           yield import('boom');\n\
         }\n\
         const it = gen();\n\
         it.next().then(r => log.push(r.value.x));\n\
         it.next().then(r => log.push(r.value.x));\n\
         it.next().then(\n\
           () => log.push('fulfilled'),\n\
           error => log.push(error)\n\
         );",
        &[
            ("a", "export var x = 42;"),
            ("b", "export var x = 39;"),
            ("boom", "throw 'foo';"),
        ],
    )
    .expect("graph evaluates");
    assert_eq!(export_log(&namespace, "log"), "42,39,foo");
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

#[test]
fn dynamic_import_from_script_shares_script_realm() {
    let mut resolver = MapResolver::new().with(
        "dep",
        "globalThis.seenFromDynamicImport = 7;\n\
         export default null;",
    );
    let value = eval_classified_with_resolver(
        "async function main() {\n\
           globalThis.seenFromDynamicImport = 0;\n\
           await import('dep');\n\
           return globalThis.seenFromDynamicImport;\n\
         }\n\
         main();",
        "main",
        Box::new(resolver.clone()),
    )
    .expect("script evaluates");
    assert_eq!(settled_fulfillment(&value), Some(Value::Number(7.0)));

    resolver = resolver.with(
        "once",
        "globalThis.dynamicImportEvaluationCount = \
           (globalThis.dynamicImportEvaluationCount || 0) + 1;\n\
         if (globalThis.dynamicImportEvaluationCount > 1) {\n\
           throw new Error('evaluated twice');\n\
         }\n\
         export default null;",
    );
    let value = eval_classified_with_resolver(
        "async function main() {\n\
           globalThis.dynamicImportEvaluationCount = 0;\n\
           await Promise.all([import('once'), import('once')]);\n\
           await import('once');\n\
           await import('once');\n\
           return globalThis.dynamicImportEvaluationCount;\n\
         }\n\
         main();",
        "main",
        Box::new(resolver),
    )
    .expect("script evaluates");
    assert_eq!(settled_fulfillment(&value), Some(Value::Number(1.0)));
}

#[test]
fn import_meta_is_null_prototype_object_in_modules() {
    let namespace = run(
        "export const proto = Object.getPrototypeOf(import.meta);\n\
         export const keys = Object.keys(import.meta).length;\n\
         export const log = [];\n\
         import(import.meta).then(\n\
           () => log.push('fulfilled'),\n\
           error => log.push(error instanceof TypeError)\n\
         );",
        &[],
    )
    .expect("graph evaluates");
    assert_eq!(export(&namespace, "proto"), Value::Null);
    assert_eq!(export(&namespace, "keys"), Value::Number(0.0));
    assert_eq!(export_log(&namespace, "log"), "true");
}

#[test]
fn import_meta_is_cached_per_module() {
    let namespace = run(
        "import { meta as depMeta, getMeta } from 'dep';\n\
         const mainMeta = import.meta;\n\
         export const sameInMain = import.meta === mainMeta && (function() { return import.meta; })() === mainMeta;\n\
         export const sameInDep = depMeta === getMeta();\n\
         export const distinct = mainMeta !== depMeta;",
        &[(
            "dep",
            "export var meta = import.meta;\n\
             export function getMeta() { return import.meta; }",
        )],
    )
    .expect("graph evaluates");
    assert_eq!(export(&namespace, "sameInMain"), Value::Boolean(true));
    assert_eq!(export(&namespace, "sameInDep"), Value::Boolean(true));
    assert_eq!(export(&namespace, "distinct"), Value::Boolean(true));
}

#[test]
fn new_import_meta_reaches_runtime_type_error() {
    let error = run("new import.meta();", &[]).expect_err("import.meta is not a constructor");
    assert_eq!(error.kind, EvalErrorKind::Runtime);
    assert!(error.message.contains("not a constructor"));
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
    assert_eq!(
        export(&namespace, "seen"),
        Value::String("done".to_owned().into())
    );
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
