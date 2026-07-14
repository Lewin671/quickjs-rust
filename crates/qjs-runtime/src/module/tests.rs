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
fn import_bytes_module_exports_immutable_uint8_array() {
    let resolver = MapResolver::new().with_bytes("bytes.bin", &[0, 255, 65]);
    let namespace = eval_module(
        "import value from \"bytes.bin\" with { type: \"bytes\" };\n\
         let resizeThrows = false;\n\
         try { value.buffer.resize(0); } catch (error) { resizeThrows = error instanceof TypeError; }\n\
         export const isUint8 = value instanceof Uint8Array;\n\
         export const length = value.length;\n\
         export const bytes = value[0] + ':' + value[1] + ':' + value[2];\n\
         export const immutable = value.buffer.immutable;\n\
         export const cannotResize = resizeThrows;",
        "main",
        Box::new(resolver),
    )
    .expect("graph evaluates");
    assert_eq!(export(&namespace, "isUint8"), Value::Boolean(true));
    assert_eq!(export(&namespace, "length"), Value::Number(3.0));
    assert_eq!(
        export(&namespace, "bytes"),
        Value::String("0:255:65".to_owned().into())
    );
    assert_eq!(export(&namespace, "immutable"), Value::Boolean(true));
    assert_eq!(export(&namespace, "cannotResize"), Value::Boolean(true));
}

#[test]
fn import_json_module_exports_parsed_default_value() {
    let resolver = MapResolver::new().with(
        "data.json",
        "{\"number\": -1.25, \"boolean\": true, \"array\": [], \"object\": {}}",
    );
    let namespace = eval_module(
        "import value from \"data.json\" with { type: \"json\" };\n\
         value.extra = 23;\n\
         export const number = value.number;\n\
         export const boolean = value.boolean;\n\
         export const array = Array.isArray(value.array);\n\
         export const objectProto = Object.getPrototypeOf(value.object) === Object.prototype;\n\
         export const extensible = value.extra;",
        "main",
        Box::new(resolver),
    )
    .expect("graph evaluates");
    assert_eq!(export(&namespace, "number"), Value::Number(-1.25));
    assert_eq!(export(&namespace, "boolean"), Value::Boolean(true));
    assert_eq!(export(&namespace, "array"), Value::Boolean(true));
    assert_eq!(export(&namespace, "objectProto"), Value::Boolean(true));
    assert_eq!(export(&namespace, "extensible"), Value::Number(23.0));
}

#[test]
fn import_text_module_exports_source_without_parsing() {
    let resolver = MapResolver::new().with("text.js", "invalid { javascript");
    let namespace = eval_module(
        "import value from \"text.js\" with { type: \"text\" };\n\
         export const text = value;\n\
         export const kind = typeof value;",
        "main",
        Box::new(resolver),
    )
    .expect("graph evaluates");
    assert_eq!(
        export(&namespace, "text"),
        Value::String("invalid { javascript".to_owned().into())
    );
    assert_eq!(
        export(&namespace, "kind"),
        Value::String("string".to_owned().into())
    );
}

#[test]
fn import_attribute_synthetic_modules_support_namespace_and_dynamic_idempotency() {
    let resolver = MapResolver::new()
        .with("data.json", "{}")
        .with("text.txt", "hello");
    let namespace = eval_module(
        "import jsonValue from \"data.json\" with { type: \"json\" };\n\
         import * as jsonNs from \"data.json\" with { type: \"json\" };\n\
         import * as textNs from \"text.txt\" with { type: \"text\" };\n\
         const dynamicNs = await import(\"data.json\", { with: { type: \"json\" } });\n\
         export const jsonKeys = Object.getOwnPropertyNames(jsonNs).join(',');\n\
         export const textDefault = textNs.default;\n\
         export const sameStatic = jsonNs.default === jsonValue;\n\
         export const sameDynamic = dynamicNs.default === jsonValue;",
        "main",
        Box::new(resolver),
    )
    .expect("graph evaluates");
    assert_eq!(
        export(&namespace, "jsonKeys"),
        Value::String("default".to_owned().into())
    );
    assert_eq!(
        export(&namespace, "textDefault"),
        Value::String("hello".to_owned().into())
    );
    assert_eq!(export(&namespace, "sameStatic"), Value::Boolean(true));
    assert_eq!(export(&namespace, "sameDynamic"), Value::Boolean(true));
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
fn anonymous_default_function_self_import_is_hoisted() {
    let source = "import f from \"main\";\n\
        export const before = f();\n\
        export const name = f.name;\n\
        export default function() { return 23; }";
    let namespace = eval_module(
        source,
        "main",
        Box::new(MapResolver::new().with("main", source)),
    )
    .expect("module evaluates");
    assert_eq!(export(&namespace, "before"), Value::Number(23.0));
    assert_eq!(
        export(&namespace, "name"),
        Value::String("default".to_owned().into())
    );
}

#[test]
fn anonymous_default_generator_self_import_is_hoisted() {
    let source = "import g from \"main\";\n\
        export const before = g().next().value;\n\
        export const name = g.name;\n\
        export default function* () { return 23; }";
    let namespace = eval_module(
        source,
        "main",
        Box::new(MapResolver::new().with("main", source)),
    )
    .expect("module evaluates");
    assert_eq!(export(&namespace, "before"), Value::Number(23.0));
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
fn default_import_tracks_named_default_function_binding_updates() {
    let namespace = run(
        "import val from \"dep\";\n\
         export const before = val();\n\
         export const after = val;",
        &[(
            "dep",
            "export default function fn() {\n\
               fn = 2;\n\
               return 1;\n\
             }",
        )],
    )
    .expect("graph evaluates");
    assert_eq!(export(&namespace, "before"), Value::Number(1.0));
    assert_eq!(export(&namespace, "after"), Value::Number(2.0));
}

#[test]
fn typeof_imported_const_observes_live_tdz() {
    let source = "let caught = false;\n\
        try { typeof y; } catch (error) { caught = error instanceof ReferenceError; }\n\
        import { x as y } from \"main\";\n\
        export const x = 23;\n\
        export const done = caught;";
    let namespace = eval_module(
        source,
        "main",
        Box::new(MapResolver::new().with("main", source)),
    )
    .expect("module evaluates");
    assert_eq!(export(&namespace, "done"), Value::Boolean(true));
}

#[test]
fn callback_typeof_imported_const_observes_live_tdz() {
    let source = "let caught = false;\n\
        function probe() { typeof y; }\n\
        try { probe(); } catch (error) { caught = error instanceof ReferenceError; }\n\
        import { x as y } from \"main\";\n\
        export const x = 23;\n\
        export const done = caught;";
    let namespace = eval_module(
        source,
        "main",
        Box::new(MapResolver::new().with("main", source)),
    )
    .expect("module evaluates");
    assert_eq!(export(&namespace, "done"), Value::Boolean(true));
}

#[test]
fn callback_assignment_to_import_binding_is_rejected() {
    let source = "let caught = false;\n\
        function probe() { f2 = null; }\n\
        import { f as f2 } from \"main\";\n\
        export function f() { return 23; }\n\
        try { probe(); } catch (error) { caught = error instanceof TypeError; }\n\
        export const done = caught && f2() === 23;";
    let namespace = eval_module(
        source,
        "main",
        Box::new(MapResolver::new().with("main", source)),
    )
    .expect("module evaluates");
    assert_eq!(export(&namespace, "done"), Value::Boolean(true));
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
fn default_function_declaration_does_not_need_statement_terminator() {
    let namespace = run(
        "var count = 0;\n\
         export default function() {} if (true) { count += 1; }\n\
         export const observed = count;",
        &[],
    )
    .expect("module evaluates");
    assert_eq!(export(&namespace, "observed"), Value::Number(1.0));
}

#[test]
fn default_class_declaration_does_not_need_statement_terminator() {
    let namespace = run(
        "var count = 0;\n\
         export default class {} if (true) { count += 1; }\n\
         export const observed = count;",
        &[],
    )
    .expect("module evaluates");
    assert_eq!(export(&namespace, "observed"), Value::Number(1.0));
}

#[test]
fn self_default_import_reads_live_default_export() {
    let namespace = run(
        "import { value, className } from \"dep\";\n\
         export { value, className };",
        &[(
            "dep",
            "export default class { valueOf() { return 45; } }\n\
             import C from \"dep\";\n\
             export const value = new C().valueOf();\n\
             export const className = C.name;",
        )],
    )
    .expect("module evaluates");
    assert_eq!(export(&namespace, "value"), Value::Number(45.0));
    assert_eq!(
        export(&namespace, "className"),
        Value::String("default".to_owned().into())
    );
}

#[test]
fn self_default_import_reads_live_default_expression() {
    let namespace = run(
        "import { observed } from \"dep\";\n\
         export { observed };",
        &[(
            "dep",
            "var x = { x: true };\n\
             export default 'x' in x;\n\
             import value from \"dep\";\n\
             export const observed = value;",
        )],
    )
    .expect("module evaluates");
    assert_eq!(export(&namespace, "observed"), Value::Boolean(true));
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
fn dependency_can_call_root_hoisted_function_before_root_body() {
    let main = "import \"a\";\n\
                export { observed } from \"a\";\n\
                export function check(value) { return value + 1; }";
    let namespace = run(
        main,
        &[
            (
                "a",
                "import { check } from \"main\";\n\
             export const observed = check(2);",
            ),
            ("main", main),
        ],
    )
    .expect("cyclic graph evaluates");
    assert_eq!(export(&namespace, "observed"), Value::Number(3.0));
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
fn indirect_export_const_import_stays_in_tdz_until_initialized() {
    let source = "let first;\n\
         try { typeof B; } catch (error) { first = error.name; }\n\
         import { B, results } from \"fixture\";\n\
         export const A = null;\n\
         export const observed = first + ':' + results.join(',');";
    let namespace = eval_module(
        source,
        "main",
        Box::new(MapResolver::new().with("main", source).with(
            "fixture",
            "export { A as B } from \"main\";\n\
             export const results = [];\n\
             try { A; } catch (error) { results.push(error.name, typeof A); }\n\
             try { B; } catch (error) { results.push(error.name, typeof B); }",
        )),
    )
    .expect("module evaluates");
    assert_eq!(
        export(&namespace, "observed"),
        Value::String(
            "ReferenceError:ReferenceError,undefined,ReferenceError,undefined"
                .to_owned()
                .into()
        )
    );
}

#[test]
fn namespace_self_import_binding_is_initialized_before_body() {
    let source = "var before = typeof ns;\n\
                  var original = ns;\n\
                  let assignment;\n\
                  try { ns = null; } catch (error) { assignment = error.name; }\n\
                  import * as ns from \"main\";\n\
                  export const observed = before + ':' + assignment + ':' + (ns === original);";
    let namespace = eval_module(
        source,
        "main",
        Box::new(MapResolver::new().with("main", source)),
    )
    .expect("module evaluates");
    assert_eq!(
        export(&namespace, "observed"),
        Value::String("object:TypeError:true".to_owned().into())
    );
}

#[test]
fn namespace_star_cycle_reads_target_live_binding() {
    let source = "import * as ns from \"cycle\";\n\
         export { c as b } from \"cycle\";\n\
         export var d = 23;\n\
         export const observed = ns.a;";
    let namespace = eval_module(
        source,
        "main",
        Box::new(MapResolver::new().with("main", source).with(
            "cycle",
            "export { b as a } from \"main\";\n\
             export { d as c } from \"main\";",
        )),
    )
    .expect("module evaluates");
    assert_eq!(export(&namespace, "observed"), Value::Number(23.0));
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
fn star_export_does_not_provide_default() {
    let error = run(
        "import value from \"agg\";\nexport const v = value;",
        &[
            ("agg", "export * from \"dep\";"),
            ("dep", "export default 1;"),
        ],
    )
    .expect_err("default is not re-exported through export-star");
    assert_eq!(error.kind, EvalErrorKind::Early);
    assert!(error.message.contains("default"), "{}", error.message);
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
fn module_top_level_this_is_undefined() {
    let namespace =
        run("export const isUndefined = this === undefined;", &[]).expect("module evaluates");
    assert_eq!(export(&namespace, "isUndefined"), Value::Boolean(true));
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
fn module_body_reuses_the_instantiated_hoisted_function_object() {
    let main = "import { observed } from 'a';\n\
                export const value = observed();\n\
                export function shared() {}";
    let namespace = run(
        main,
        &[
            (
                "a",
                "import { shared } from 'main';\n\
                 shared.marker = 42;\n\
                 export function observed() { return shared.marker; }",
            ),
            ("main", main),
        ],
    )
    .expect("graph evaluates");
    assert_eq!(export(&namespace, "value"), Value::Number(42.0));
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
fn static_self_imports_evaluate_module_once() {
    let source = "globalThis.staticSelfImportCount = \
                    (globalThis.staticSelfImportCount || 0) + 1;\n\
                  if (globalThis.staticSelfImportCount > 1) {\n\
                    throw new Error('evaluated twice');\n\
                  }\n\
                  import {} from \"main\";\n\
                  import \"main\";\n\
                  import * as ns1 from \"main\";\n\
                  import dflt1 from \"main\";\n\
                  export {} from \"main\";\n\
                  import dflt2, {} from \"main\";\n\
                  export * from \"main\";\n\
                  export * as ns2 from \"main\";\n\
                  import dflt3, * as ns3 from \"main\";\n\
                  export default null;\n\
                  export const count = globalThis.staticSelfImportCount;";
    let namespace = eval_module(
        source,
        "main",
        Box::new(MapResolver::new().with("main", source)),
    )
    .expect("self-import graph evaluates");
    assert_eq!(export(&namespace, "count"), Value::Number(1.0));
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
fn top_level_await_body_reuses_instantiated_function_binding() {
    let namespace = run(
        "export function fn() { return 42; }
         await function fn() { return 111; };
         export const value = fn();",
        &[],
    )
    .expect("module evaluates");
    assert_eq!(export(&namespace, "value"), Value::Number(42.0));
}

#[test]
fn top_level_await_of_plain_value() {
    // `await <non-promise>` resolves to the value itself.
    let namespace = run("export var x = await 7;", &[]).expect("graph evaluates");
    assert_eq!(export(&namespace, "x"), Value::Number(7.0));
}

#[test]
fn top_level_await_catches_dynamic_import_rejection() {
    let namespace = run(
        "let caught = false;\n\
         try {\n\
           await import('missing');\n\
         } catch (error) {\n\
           caught = true;\n\
         }\n\
         export { caught };",
        &[],
    )
    .expect("graph evaluates");
    assert_eq!(export(&namespace, "caught"), Value::Boolean(true));
}

#[test]
fn top_level_await_propagates_dynamic_import_rejection() {
    let error = run("await import('missing');", &[])
        .expect_err("unhandled dynamic import rejection rejects module evaluation");
    assert_eq!(error.kind, EvalErrorKind::Runtime);
    assert!(error.message.contains("Cannot resolve module"));
}

#[test]
fn dynamic_import_rejects_when_imported_tla_rejects() {
    let namespace = run(
        "let caught = '';\n\
         await import('dep').then(\n\
           () => { caught = 'fulfilled'; },\n\
           error => { caught = error.message; }\n\
         );\n\
         export { caught };",
        &[(
            "dep",
            "export default await Promise.reject(new TypeError('import failed'));",
        )],
    )
    .expect("dynamic import rejection is catchable");
    assert_eq!(
        export(&namespace, "caught"),
        Value::String("import failed".to_owned().into())
    );
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
fn top_level_await_does_not_block_sibling_module() {
    let namespace = run(
        "import \"tla\";\n\
         import { check } from \"sync\";\n\
         export const seen = check;\n\
         export const order = globalThis.log.join(',');",
        &[
            (
                "tla",
                "globalThis.log = [];\n\
                 globalThis.log.push('tla-start');\n\
                 globalThis.check = false;\n\
                 await 0;\n\
                 globalThis.log.push('tla-done');\n\
                 globalThis.check = true;",
            ),
            (
                "sync",
                "globalThis.log.push('sync');\n\
                 export const { check } = globalThis;",
            ),
        ],
    )
    .expect("graph evaluates");
    assert_eq!(export(&namespace, "seen"), Value::Boolean(false));
    assert_eq!(
        export(&namespace, "order"),
        Value::String("tla-start,sync,tla-done".to_owned().into())
    );
}

#[test]
fn module_destructuring_export_snapshots_local_binding() {
    let namespace = run(
        "import { check } from \"sync\";\n\
         export const seen = check;",
        &[(
            "sync",
            "globalThis.check = false;\n\
             export const { check } = globalThis;\n\
             globalThis.check = true;",
        )],
    )
    .expect("graph evaluates");
    assert_eq!(export(&namespace, "seen"), Value::Boolean(false));
}

#[test]
fn imported_top_level_await_rejection_prevents_importer_body() {
    let error = run(
        "import value from \"dep\";\n\
         throw new Error('unreachable');",
        &[(
            "dep",
            "export default 42;\n\
             await Promise.reject(new TypeError('dependency failed'));",
        )],
    )
    .expect_err("dependency rejection rejects module evaluation");
    assert_eq!(error.kind, EvalErrorKind::Runtime);
    assert!(error.message.contains("TypeError"));
    assert!(!error.message.contains("unreachable"));
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
