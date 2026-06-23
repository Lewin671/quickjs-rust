use crate::{Value, eval};

#[test]
fn evaluates_boolean_builtins() {
    assert_eq!(
        eval("typeof Boolean;"),
        Ok(Value::String("function".to_owned().into()))
    );
    assert_eq!(eval("Boolean.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Boolean();"), Ok(Value::Boolean(false)));
    assert_eq!(eval("Boolean(0);"), Ok(Value::Boolean(false)));
    assert_eq!(eval("Boolean(1);"), Ok(Value::Boolean(true)));
    assert_eq!(eval("Boolean('');"), Ok(Value::Boolean(false)));
    assert_eq!(eval("Boolean('x');"), Ok(Value::Boolean(true)));
    assert_eq!(
        eval("Boolean.prototype.constructor === Boolean;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Boolean.prototype.toString.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("Boolean.prototype.valueOf.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("Boolean.prototype.toString();"),
        Ok(Value::String("false".to_owned().into()))
    );
    assert_eq!(
        eval("Boolean.prototype.valueOf();"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval("true.toString();"),
        Ok(Value::String("true".to_owned().into()))
    );
    assert_eq!(eval("false.valueOf();"), Ok(Value::Boolean(false)));
    assert_eq!(
        eval("(new Boolean(true)).toString();"),
        Ok(Value::String("true".to_owned().into()))
    );
    assert_eq!(
        eval("(new Boolean(0)).valueOf();"),
        Ok(Value::Boolean(false))
    );
    assert!(eval("let o = Object.create(Boolean.prototype); o.valueOf();").is_err());
}

#[test]
fn primitive_wrapper_constructors_use_new_target_realm_default_prototype() {
    for (name, marker) in [
        ("Boolean", "__quickjsRustRealmBooleanPrototype"),
        ("Number", "__quickjsRustRealmNumberPrototype"),
        ("String", "__quickjsRustRealmStringPrototype"),
    ] {
        assert_eq!(
            eval(&format!(
                "let realmPrototype = {{}}; \
                 function C() {{}} \
                 Object.defineProperty(C, '{marker}', {{ value: realmPrototype }}); \
                 C.prototype = null; \
                 Object.getPrototypeOf(Reflect.construct({name}, [], C)) === realmPrototype;"
            )),
            Ok(Value::Boolean(true)),
            "{name} should use the marked newTarget realm prototype"
        );
    }
}

#[test]
fn evaluates_global_undefined_binding() {
    assert_eq!(eval("undefined;"), Ok(Value::Undefined));
    assert_eq!(eval("undefined === undefined;"), Ok(Value::Boolean(true)));
    assert_eq!(
        eval(
            "let d = Object.getOwnPropertyDescriptor(this, 'undefined'); d.writable + ':' + d.enumerable + ':' + d.configurable;"
        ),
        Ok(Value::String("false:false:false".to_owned().into()))
    );
}

#[test]
fn eval_comment_only_sources_return_undefined() {
    assert_eq!(eval("eval('// comment only');"), Ok(Value::Undefined));
    assert_eq!(
        eval("eval('/* comment\\nonly */ /* second */');"),
        Ok(Value::Undefined)
    );
}

#[test]
fn eval_line_comment_with_line_terminator_still_evaluates_tail() {
    assert_eq!(
        eval("var yy = 0; eval('// ignored\\nyy = -1'); yy;"),
        Ok(Value::Number(-1.0))
    );
    assert_eq!(
        eval("var yy = 0; eval('// ignored\\u2028yy = -1'); yy;"),
        Ok(Value::Number(-1.0))
    );
}

#[test]
fn global_nan_is_non_writable() {
    // Sloppy mode: assignment silently fails, NaN remains a number.
    assert_eq!(
        eval("NaN = true; typeof NaN;"),
        Ok(Value::String("number".to_owned().into()))
    );
    assert_eq!(eval("NaN = true; NaN !== NaN;"), Ok(Value::Boolean(true)));
    assert_eq!(
        eval("var NaN = 1.0; NaN = 'asdf'; NaN = true; NaN !== NaN;"),
        Ok(Value::Boolean(true))
    );
    // Descriptor check.
    assert_eq!(
        eval(
            "let d = Object.getOwnPropertyDescriptor(this, 'NaN'); d.writable + ':' + d.enumerable + ':' + d.configurable;"
        ),
        Ok(Value::String("false:false:false".to_owned().into()))
    );
    // Strict mode: TypeError on assignment to non-writable NaN.
    assert!(eval("'use strict'; NaN = 1;").is_err());
    assert_eq!(
        eval(
            "let caught = false; try { (function() { 'use strict'; NaN = 1; })(); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn global_infinity_is_non_writable() {
    // Sloppy mode: assignment silently fails, Infinity remains a number.
    assert_eq!(
        eval("Infinity = true; typeof Infinity;"),
        Ok(Value::String("number".to_owned().into()))
    );
    assert_eq!(
        eval("Infinity = true; Infinity === 1/0;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("var Infinity = 1.0; Infinity = 'asdf'; Infinity = true; Infinity === 1/0;"),
        Ok(Value::Boolean(true))
    );
    // Descriptor check.
    assert_eq!(
        eval(
            "let d = Object.getOwnPropertyDescriptor(this, 'Infinity'); d.writable + ':' + d.enumerable + ':' + d.configurable;"
        ),
        Ok(Value::String("false:false:false".to_owned().into()))
    );
    // Strict mode: TypeError on assignment to non-writable Infinity.
    assert!(eval("'use strict'; Infinity = 1;").is_err());
    assert_eq!(
        eval(
            "let caught = false; try { (function() { 'use strict'; Infinity = 1; })(); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("function harness() {} Infinity = true; typeof Infinity + ':' + Infinity;"),
        Ok(Value::String("number:Infinity".to_owned().into()))
    );
}

#[test]
fn global_undefined_is_non_writable() {
    // Sloppy mode: assignment silently fails, undefined stays undefined.
    assert_eq!(
        eval("undefined = true; typeof undefined;"),
        Ok(Value::String("undefined".to_owned().into()))
    );
    // Strict mode: TypeError on assignment to non-writable undefined.
    assert!(eval("'use strict'; undefined = 1;").is_err());
    assert_eq!(
        eval(
            "let caught = false; try { (function() { 'use strict'; undefined = 1; })(); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("function harness() {} undefined = 5; typeof undefined + ':' + undefined;"),
        Ok(Value::String("undefined:undefined".to_owned().into()))
    );
    assert_eq!(
        eval(
            "function harness() {} let caught = false; try { (function() { 'use strict'; undefined = 1; })(); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn evaluates_global_this_binding() {
    assert_eq!(eval("globalThis === this;"), Ok(Value::Boolean(true)));
    assert_eq!(
        eval("globalThis.Object === Object;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("this.globalThis === globalThis;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Object.getOwnPropertyDescriptor(this, 'globalThis').writable;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Object.getOwnPropertyDescriptor(this, 'globalThis').enumerable;"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval("Object.getOwnPropertyDescriptor(this, 'globalThis').configurable;"),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn indirect_eval_uses_marked_dynamic_realm_global() {
    assert_eq!(
        eval(
            "var __quickjsRustDynamicFunctionRealm; \
             var other = Object.create(globalThis); \
             other.eval = function(source) { \
               var previousRealm = __quickjsRustDynamicFunctionRealm; \
               __quickjsRustDynamicFunctionRealm = other; \
               globalThis.__quickjsRustDynamicFunctionRealm = other; \
               try { return (0, eval)(source); } \
               finally { \
                 __quickjsRustDynamicFunctionRealm = previousRealm; \
                 globalThis.__quickjsRustDynamicFunctionRealm = previousRealm; \
               } \
             }; \
             var otherEval = other.eval; \
             other.value = 7; \
             var read = otherEval('value'); \
             otherEval('var x = 23;'); \
             [read, typeof x, other.x].join(':');"
        ),
        Ok(Value::String("7:undefined:23".to_owned().into()))
    );
}

#[test]
fn cross_realm_eval_binding_is_not_direct_eval() {
    assert_eq!(
        eval(
            "var __quickjsRustDynamicFunctionRealm; \
             var other = Object.create(globalThis); \
             other.eval = function(source) { \
               var previousRealm = __quickjsRustDynamicFunctionRealm; \
               __quickjsRustDynamicFunctionRealm = other; \
               globalThis.__quickjsRustDynamicFunctionRealm = other; \
               try { return (0, eval)(source); } \
               finally { \
                 __quickjsRustDynamicFunctionRealm = previousRealm; \
                 globalThis.__quickjsRustDynamicFunctionRealm = previousRealm; \
               } \
             }; \
             var x = 'outside'; \
             var result; \
             (function() { \
               var eval = other.eval; \
               eval('var x = \"inside\";'); \
               result = x; \
             }()); \
             [result, typeof x, other.x].join(':');"
        ),
        Ok(Value::String("outside:string:inside".to_owned().into()))
    );
}

#[test]
fn indirect_eval_uses_marked_realm_for_primitive_prototypes() {
    assert_eq!(
        eval(
            "var __quickjsRustDynamicFunctionRealm; \
             var other = Object.create(globalThis); \
             other.eval = function(source) { \
               var previousRealm = __quickjsRustDynamicFunctionRealm; \
               __quickjsRustDynamicFunctionRealm = other; \
               globalThis.__quickjsRustDynamicFunctionRealm = other; \
               try { return (0, eval)(source); } \
               finally { \
                 __quickjsRustDynamicFunctionRealm = previousRealm; \
                 globalThis.__quickjsRustDynamicFunctionRealm = previousRealm; \
               } \
             }; \
             other.Number = function Number() {}; \
             other.Number.prototype = Object.create(Number.prototype); \
             other.Number.prototype.test262 = 'number prototype'; \
             other.value = 1; \
             other.eval('value.test262');"
        ),
        Ok(Value::String("number prototype".to_owned().into()))
    );
}

#[test]
fn indirect_eval_uses_marked_realm_for_symbol_primitive_set() {
    assert_eq!(
        eval(
            "var __quickjsRustDynamicFunctionRealm; \
             var other = Object.create(globalThis); \
             other.eval = function(source) { \
               var previousRealm = __quickjsRustDynamicFunctionRealm; \
               __quickjsRustDynamicFunctionRealm = other; \
               globalThis.__quickjsRustDynamicFunctionRealm = other; \
               try { return (0, eval)(source); } \
               finally { \
                 __quickjsRustDynamicFunctionRealm = previousRealm; \
                 globalThis.__quickjsRustDynamicFunctionRealm = previousRealm; \
               } \
             }; \
             var intrinsicSymbol = Symbol; \
             other.Symbol = function Symbol(description) { return intrinsicSymbol(description); }; \
             other.Symbol.prototype = Object.create(Symbol.prototype); \
             var count = 0; \
             var spy = new Proxy({}, { set: function() { count += 1; return true; } }); \
             Object.setPrototypeOf(other.Symbol.prototype, spy); \
             other.eval('Symbol().test262 = null;'); \
             count;"
        ),
        Ok(Value::Number(1.0))
    );
}

#[test]
fn exposes_print_host_global() {
    assert_eq!(
        eval("typeof print;"),
        Ok(Value::String("function".to_owned().into()))
    );
    assert_eq!(eval("print.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("this.print === print;"), Ok(Value::Boolean(true)));
    assert_eq!(
        eval("Object.getOwnPropertyDescriptor(this, 'print').enumerable;"),
        Ok(Value::Boolean(false))
    );
    // Returns undefined regardless of argument count, and is reachable from a
    // nested call frame (the $DONE async channel prints from a reaction).
    assert_eq!(eval("print('quiet', 1, 2);"), Ok(Value::Undefined));
    assert_eq!(
        eval("(function () { return print('nested'); })();"),
        Ok(Value::Undefined)
    );
}

#[test]
fn evaluates_test262_same_value_host_helper() {
    assert_eq!(
        eval("__quickjsRustAssertSameValue(NaN, NaN);"),
        Ok(Value::Undefined)
    );
    assert!(eval("__quickjsRustAssertSameValue(+0, -0, 'zero');").is_err());
}

#[test]
fn evaluates_global_eval_builtin() {
    assert_eq!(
        eval("typeof eval;"),
        Ok(Value::String("function".to_owned().into()))
    );
    assert_eq!(eval("eval.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("this.eval === eval;"), Ok(Value::Boolean(true)));
    assert_eq!(
        eval(
            "let d = Object.getOwnPropertyDescriptor(this, 'eval'); (d.value === eval) + ':' + d.writable + ':' + d.enumerable + ':' + d.configurable;"
        ),
        Ok(Value::String("true:true:false:true".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let d = Object.getOwnPropertyDescriptor(this, 'Object'); (d.value === Object) + ':' + d.writable + ':' + d.enumerable + ':' + d.configurable;"
        ),
        Ok(Value::String("true:true:false:true".to_owned().into()))
    );
    assert_eq!(eval("eval(7);"), Ok(Value::Number(7.0)));
    assert_eq!(eval("eval('1 + 2;');"), Ok(Value::Number(3.0)));
    assert_eq!(
        eval("let value = 1; eval('value = value + 2;'); value;"),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval("eval('var leaked = 1;'); leaked;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("eval('{ let lexical = 1; }'); typeof lexical;"),
        Ok(Value::String("undefined".to_owned().into()))
    );
    assert!(eval("eval('{ let lexical = 1; } lexical;')").is_err());
    assert!(eval("eval('{ let f = 123; { function f() {} } } f;')").is_err());
    assert!(
        eval("eval('{ let f = 123; if (true) function f() {} else function _f() {} } f;')")
            .is_err()
    );
    assert!(eval("eval('for (let i = 0; i < 1; i++) {} i;')").is_err());
    assert!(eval("eval('for (let f; ; ) { { function f() {} } break; } f;')").is_err());
    assert!(eval("eval('for (let k in { a: 1 }) {} k;')").is_err());
    assert!(eval("eval('for (let v of [1]) {} v;')").is_err());
    assert!(eval("eval('switch (1) { case 1: let s = 1; } s;')").is_err());
    assert_eq!(
        eval(
            "eval('var before = f; { function f() { return 7; } } before === undefined && f() === 7;');"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "{ function evalLexCollisionFn() {} } \
             let caught = false; \
             try { eval('var evalLexCollisionVar; let evalLexCollisionFn;'); } catch (error) { caught = error instanceof SyntaxError; } \
             caught + ':' + (typeof evalLexCollisionVar);"
        ),
        Ok(Value::String("true:undefined".to_owned().into()))
    );
    // A direct eval gets its own lexical environment, so a `let` whose name
    // matches an outer *lexical* binding is distinct, not a conflict, and never
    // leaks out of the eval.
    assert_eq!(
        eval("let distinctOuter = 23; eval('let distinctOuter = 1;'); distinctOuter;"),
        Ok(Value::Number(23.0))
    );
    assert_eq!(
        eval("eval('let evalLocalLexical = 3;'); typeof evalLocalLexical;"),
        Ok(Value::String("undefined".to_owned().into()))
    );
    // A direct eval resolves a name to the innermost active block lexical, even
    // when that binding shadows a same-named outer/function binding (the block
    // binding is stored under a mangled name; eval must still find it).
    assert_eq!(
        eval("function f() { let w = 'outer'; { let w = 'inner'; return eval('w'); } } f();"),
        Ok(Value::String("inner".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let lexHeritage = 'outside'; let r; { let lexHeritage = 'inside'; r = eval('lexHeritage'); } r;"
        ),
        Ok(Value::String("inside".to_owned().into()))
    );
    assert_eq!(
        eval(
            "var directWith = 'global'; \
             function testcase() { \
                 var object = { directWith: 'with' }; \
                 var directWith = 'local'; \
                 with (object) { return eval('directWith'); } \
             } \
             testcase();"
        ),
        Ok(Value::String("with".to_owned().into()))
    );
}

#[test]
fn strict_direct_eval_declarations_stay_eval_local() {
    assert_eq!(
        eval(
            "'use strict'; \
             function testcase() { \
                 var value = 0; \
                 eval('var value = 1;'); \
                 return value; \
             } \
             testcase();"
        ),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval(
            "'use strict'; \
             function testcase() { \
                 eval('function evalLocalFunction(x) { return x; }'); \
                 return typeof evalLocalFunction; \
             } \
             testcase();"
        ),
        Ok(Value::String("undefined".to_owned().into()))
    );
    assert_eq!(
        eval(
            "function testcase() { \
                 eval('\"use strict\"; var sourceStrictValue = 1;'); \
                 return typeof sourceStrictValue; \
             } \
             testcase();"
        ),
        Ok(Value::String("undefined".to_owned().into()))
    );
    assert_eq!(
        eval(
            "function testcase() { \
                 eval('\"use strict\"; function sourceStrictFunction(x) { return x; }'); \
                 return typeof sourceStrictFunction; \
             } \
             testcase();"
        ),
        Ok(Value::String("undefined".to_owned().into()))
    );
}

#[test]
fn strict_direct_eval_assignments_write_back_to_caller() {
    assert_eq!(
        eval(
            "'use strict'; \
             function testcase() { \
                 var value = 0; \
                 eval('value = 2;'); \
                 return value; \
             } \
             testcase();"
        ),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval(
            "function testcase() { \
                 var value = 0; \
                 eval('\"use strict\"; value = 3;'); \
                 return value; \
             } \
             testcase();"
        ),
        Ok(Value::Number(3.0))
    );
}

#[test]
fn sloppy_global_eval_validates_global_declarations_before_instantiation() {
    assert_eq!(
        eval(
            "let error; \
             try { eval('function NaN() {}'); } catch (caught) { error = caught; } \
             error instanceof TypeError;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "try { eval('var shouldNotBeDefined; function NaN() {}'); } catch (caught) {} \
             Object.getOwnPropertyDescriptor(this, 'shouldNotBeDefined') === undefined;"
        ),
        Ok(Value::Boolean(true))
    );
    assert!(
        eval(
            "let evalGlobalLexCollision; \
             eval('var evalGlobalLexCollision;');"
        )
        .is_err()
    );
}

#[test]
fn sloppy_global_eval_function_binding_updates_configurable_property_descriptor() {
    assert_eq!(
        eval(
            "Object.defineProperty(this, 'evalConfigurableFunction', { \
                 enumerable: false, writable: false, configurable: true \
             }); \
             let initial = null; \
             eval('initial = evalConfigurableFunction; function evalConfigurableFunction() { return 345; }'); \
             let descriptor = Object.getOwnPropertyDescriptor(this, 'evalConfigurableFunction'); \
             (typeof initial) + ':' + initial() + ':' + descriptor.writable + ':' + descriptor.enumerable + ':' + descriptor.configurable;"
        ),
        Ok(Value::String(
            "function:345:true:true:true".to_owned().into()
        ))
    );
}

#[test]
fn evaluates_global_eval_pure_regexp_literals() {
    assert_eq!(
        eval(
            "let RegExp = function() { throw new Error('shadowed'); }; eval('/\\\\u0041/i').source + ':' + eval('/a/i').ignoreCase;"
        ),
        Ok(Value::String("\\u0041:true".to_owned().into()))
    );
    assert_eq!(
        eval("eval('/[\\\\/]/').test('/');"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(eval("eval('  /a/g;  ').global;"), Ok(Value::Boolean(true)));
    assert_eq!(eval("eval('/a/; 1');"), Ok(Value::Number(1.0)));
    assert_eq!(eval("eval('// comment');"), Ok(Value::Undefined));
}

#[test]
fn evaluates_direct_eval_annex_b_function_bindings_in_function_frames() {
    assert_eq!(
        eval(
            "var init, changed; \
             (function() { eval('init = f; f = 123; changed = f; { function f() {} }'); }()); \
             String(init) + ':' + changed + ':' + typeof f;"
        ),
        Ok(Value::String("undefined:123:undefined".to_owned().into()))
    );
    assert_eq!(
        eval(
            "var init, after; \
             (function(f) { eval('init = f; { function f() {} } after = typeof f;'); }(123)); \
             init + ':' + after + ':' + typeof f;"
        ),
        Ok(Value::String("123:function:undefined".to_owned().into()))
    );
}

#[test]
fn eval_annex_b_function_declarations_capture_block_scoped_binding() {
    assert_eq!(
        eval(
            "var initialBV, currentBV; \
             eval('{ function f() { initialBV = f; f = 123; currentBV = f; return \"decl\"; } }'); \
             f(); \
             initialBV() + ':' + currentBV + ':' + f();"
        ),
        Ok(Value::String("decl:123:decl".to_owned().into()))
    );
    assert_eq!(
        eval(
            "var initialBV, currentBV; \
             eval('if (true) function f() { initialBV = f; f = 123; currentBV = f; return \"decl\"; } else function _f() {}'); \
             f(); \
             initialBV() + ':' + currentBV + ':' + f();"
        ),
        Ok(Value::String("decl:123:decl".to_owned().into()))
    );
    assert_eq!(
        eval(
            "var initialBV, currentBV; \
             eval('switch (1) { case 1: function f() { initialBV = f; f = 123; currentBV = f; return \"decl\"; } }'); \
             f(); \
             initialBV() + ':' + currentBV + ':' + f();"
        ),
        Ok(Value::String("decl:123:decl".to_owned().into()))
    );
}

#[test]
fn evaluates_global_eval_annex_b_bindings_as_configurable() {
    assert_eq!(
        eval(
            "eval('if (true) { function test262Fn() {} }'); \
             let d = Object.getOwnPropertyDescriptor(this, 'test262Fn'); \
             typeof test262Fn + ':' + d.configurable;"
        ),
        Ok(Value::String("function:true".to_owned().into()))
    );
}

#[test]
fn sloppy_direct_eval_allows_annex_b_catch_parameter_redeclarations() {
    assert_eq!(
        eval(
            "try { throw null; } catch (err) { \
                 eval('function err() {}'); \
                 eval('function* err() {}'); \
                 eval('async function err() {}'); \
                 eval('async function* err() {}'); \
                 eval('var err;'); \
                 eval('for (var err; false; ) {}'); \
                 eval('for (var err in []) {}'); \
                 eval('for (var err of []) {}'); \
             } \
             'done';"
        ),
        Ok(Value::String("done".to_owned().into()))
    );
    assert!(
        eval(
            "{ let blocked; \
               try { eval('var blocked;'); } catch (error) { throw error; } \
             }"
        )
        .is_err(),
        "ordinary block lexicals must still reject direct eval var redeclarations"
    );
}

#[test]
fn evaluates_indirect_eval_against_global_scope() {
    assert_eq!(
        eval(
            "let local = 1; \
             (function() { let local = 2; return (0, eval)('typeof local'); }());"
        ),
        Ok(Value::String("number".to_owned().into()))
    );
    // Indirect eval evaluates lexical declarations in a fresh declarative
    // environment that is discarded afterwards: the binding neither persists as
    // a referenceable name nor becomes an own property of the global object.
    assert_eq!(
        eval(
            "(function(source) { return (0, eval)(source); }('let indirectLexical = 1;')); \
             Object.prototype.hasOwnProperty.call(this, 'indirectLexical');"
        ),
        Ok(Value::Boolean(false))
    );
    assert!(
        eval(
            "(function(source) { return (0, eval)(source); }('let indirectLexical = 1;')); \
             indirectLexical;"
        )
        .is_err(),
        "indirect eval lexical binding must not leak into global scope"
    );
    assert_eq!(
        eval(
            "let outside = 23; \
             (0, eval)('let outside;'); \
             (0, eval)('\"use strict\"; let outside;'); \
             outside;"
        ),
        Ok(Value::Number(23.0))
    );
    assert_eq!(
        eval(
            "(0, eval)('let xNonStrict = 3;'); \
             typeof xNonStrict + ':' + Object.prototype.hasOwnProperty.call(this, 'xNonStrict');"
        ),
        Ok(Value::String("undefined:false".to_owned().into()))
    );
    assert_eq!(
        eval(
            "(0, eval)('\"use strict\"; var strictIndirectVar = 88;'); \
             typeof strictIndirectVar + ':' + Object.prototype.hasOwnProperty.call(this, 'strictIndirectVar');"
        ),
        Ok(Value::String("undefined:false".to_owned().into()))
    );
    assert_eq!(
        eval(
            "(0, eval)('\"use strict\"; function strictIndirectFn(){}'); \
             typeof strictIndirectFn + ':' + Object.prototype.hasOwnProperty.call(this, 'strictIndirectFn');"
        ),
        Ok(Value::String("undefined:false".to_owned().into()))
    );
    assert_eq!(
        eval(
            "var nestedIndirect = 'global'; \
             function fn() { \
               var nestedIndirect = 'local'; \
               return eval('var indirect = eval; var nestedIndirect = \"eval\"; indirect(\"nestedIndirect\");'); \
             } \
             fn();"
        ),
        Ok(Value::String("global".to_owned().into()))
    );
}

#[test]
fn optional_eval_call_is_indirect_eval_against_global_scope() {
    assert_eq!(
        eval(
            "const a = 'global'; \
             function fn() { const a = 'local'; return eval?.('a'); } \
             fn() + ':' + Object.prototype.hasOwnProperty.call(this, 'a');"
        ),
        Ok(Value::String("global:false".to_owned().into()))
    );
    assert_eq!(
        eval("const b = 'global'; ((b) => eval?.('b'))('local');"),
        Ok(Value::String("global".to_owned().into()))
    );
}

#[test]
fn eval_script_host_evaluates_global_script() {
    // __quickjsRustEvalScript ($262.evalScript) runs a global script: top-level
    // `let`/`const`/`class` become persistent, referenceable global lexical
    // bindings that are not own properties of the global object.
    assert_eq!(
        eval(
            "__quickjsRustEvalScript('let scriptLexical = 5;'); \
             scriptLexical + ':' + Object.prototype.hasOwnProperty.call(this, 'scriptLexical');"
        ),
        Ok(Value::String("5:false".to_owned().into()))
    );
    // var/function declarations reach the global var environment (own property).
    assert_eq!(
        eval(
            "__quickjsRustEvalScript('var scriptVar = 9;'); \
             scriptVar + ':' + Object.prototype.hasOwnProperty.call(this, 'scriptVar');"
        ),
        Ok(Value::String("9:true".to_owned().into()))
    );
    // A sloppy Annex B block-function hoisted by eval is configurable, so a
    // later global lexical declaration of the same name does not collide.
    assert_eq!(
        eval(
            "eval('if (true) { function collide() {} }'); \
             __quickjsRustEvalScript('let collide = 1;'); collide;"
        ),
        Ok(Value::Number(1.0))
    );
}

#[test]
fn initializes_global_hoisted_bindings_before_script_execution() {
    assert_eq!(
        eval("var before = f; { function f() { return 9; } } before === undefined && f() === 9;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "{ function f() {} } var d = Object.getOwnPropertyDescriptor(this, 'f'); d.enumerable + ':' + d.writable + ':' + d.configurable;"
        ),
        Ok(Value::String("true:true:false".to_owned().into()))
    );
    assert_eq!(
        eval(
            "eval(\"Object.defineProperty(this, 'annexGlobalFn', { value: 'x', enumerable: false, writable: true, configurable: true });\"); \
             eval(\"{ function annexGlobalFn() { return 9; } }\"); \
             let d = Object.getOwnPropertyDescriptor(this, 'annexGlobalFn'); \
             annexGlobalFn() + ':' + d.enumerable + ':' + d.writable + ':' + d.configurable;"
        ),
        Ok(Value::String("9:false:true:true".to_owned().into()))
    );
}

#[test]
fn skips_annex_b_function_binding_for_parameter_collisions() {
    assert_eq!(
        eval(
            "var init, after; (function(f) { init = f; if (false) function _f() {} else function f() {} after = f; }(123)); init + ':' + after;"
        ),
        Ok(Value::String("123:123".to_owned().into()))
    );
    assert_eq!(
        eval(
            "var init, after; (function(f = 123) { init = f; if (false) function _f() {} else function f() {} after = f; }()); init + ':' + after;"
        ),
        Ok(Value::String("123:123".to_owned().into()))
    );
}

#[test]
fn evaluates_uri_coding_builtins() {
    assert_eq!(eval("encodeURI.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("decodeURI.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("encodeURIComponent.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("decodeURIComponent.length;"), Ok(Value::Number(1.0)));
    assert_eq!(
        eval(
            "let d = Object.getOwnPropertyDescriptor(this, 'decodeURIComponent'); (d.value === decodeURIComponent) + ':' + d.writable + ':' + d.enumerable + ':' + d.configurable;"
        ),
        Ok(Value::String("true:true:false:true".to_owned().into()))
    );
    assert_eq!(
        eval("encodeURI('https://example.test/a b?x=1&y=\\u00E9#frag');"),
        Ok(Value::String(
            "https://example.test/a%20b?x=1&y=%C3%A9#frag"
                .to_owned()
                .into()
        ))
    );
    assert_eq!(
        eval("encodeURIComponent('a b?x=1&y=\\u00E9');"),
        Ok(Value::String(
            "a%20b%3Fx%3D1%26y%3D%C3%A9".to_owned().into()
        ))
    );
    assert_eq!(
        eval("decodeURI('https://example.test/a%20b?x=1&y=%C3%A9%23frag');"),
        Ok(Value::String(
            "https://example.test/a b?x=1&y=\u{00E9}%23frag"
                .to_owned()
                .into()
        ))
    );
    assert_eq!(
        eval("decodeURIComponent('a%20b%3Fx%3D1%26y%3D%C3%A9');"),
        Ok(Value::String("a b?x=1&y=\u{00E9}".to_owned().into()))
    );
    assert_eq!(
        eval("encodeURIComponent(String.fromCodePoint(0x1D306));"),
        Ok(Value::String("%F0%9D%8C%86".to_owned().into()))
    );
    assert_eq!(
        eval("encodeURIComponent(String.fromCharCode(0xD834, 0xDF06));"),
        Ok(Value::String("%F0%9D%8C%86".to_owned().into()))
    );
    assert_eq!(
        eval("encodeURIComponent(decodeURIComponent('%F0%9D%8C%86'));"),
        Ok(Value::String("%F0%9D%8C%86".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let caught = false; try { decodeURIComponent('%E0%A4%A'); } catch (error) { caught = error instanceof URIError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn evaluates_annex_b_escape_builtins() {
    assert_eq!(eval("escape.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("unescape.length;"), Ok(Value::Number(1.0)));
    assert_eq!(
        eval("escape('');"),
        Ok(Value::String(::std::rc::Rc::new(String::new())))
    );
    assert_eq!(
        eval("escape('AZaz09@*_+-./');"),
        Ok(Value::String("AZaz09@*_+-./".to_owned().into()))
    );
    assert_eq!(
        eval("escape(' #éĀ');"),
        Ok(Value::String("%20%23%E9%u0100".to_owned().into()))
    );
    assert_eq!(
        eval("escape(String.fromCodePoint(0x1D306));"),
        Ok(Value::String("%uD834%uDF06".to_owned().into()))
    );
    assert_eq!(
        eval("unescape('%20%23%E9%u0100');"),
        Ok(Value::String(" #éĀ".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let value = unescape('%uD834%uDF06'); value.length === 2 && value.charCodeAt(0) === 0xD834 && value.charCodeAt(1) === 0xDF06;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("unescape('%zz%u12xz');"),
        Ok(Value::String("%zz%u12xz".to_owned().into()))
    );
}

#[test]
fn keeps_global_object_properties_and_bindings_in_sync() {
    assert_eq!(
        eval("let global = Function('return this;')(); global === this;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("let global = Function('return this;')(); global.customGlobal = 7; customGlobal;"),
        Ok(Value::Number(7.0))
    );
    assert_eq!(
        eval(
            "let global = Function('return this;')(); global.Object = function FakeObject() {}; Object === global.Object;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "toString = Object.prototype.toString; typeof toString + ':' + typeof this.toString + ':' + this.toString();"
        ),
        Ok(Value::String(
            "function:function:[object Object]".to_owned().into()
        ))
    );
    assert_eq!(
        eval(
            "Object.defineProperty(Object.prototype, 'prop', { value: 1001, writable: false, configurable: false }); var prop = 1002; this.hasOwnProperty('prop') + ':' + prop + ':' + this.prop;"
        ),
        Ok(Value::String("true:1002:1002".to_owned().into()))
    );
    assert_eq!(
        eval("function f() { var localOnly = 1; return this.hasOwnProperty('localOnly'); } f();"),
        Ok(Value::Boolean(false))
    );
}

#[test]
fn direct_function_eval_rejects_var_arguments_only_in_parameter_scope() {
    // A direct eval in a *parameter default* may not hoist a `var`/`function`
    // named `arguments`: with parameter expressions the parameter list has its
    // own environment binding `arguments`, which the eval's separate var
    // declaration collides with (EvalDeclarationInstantiation SyntaxError).
    assert!(
        eval("function f(p = eval('var arguments = 1')) {} f();").is_err(),
        "parameter-scope eval declaring var arguments must throw"
    );
    assert!(
        eval("function f(p = eval('function arguments() {}')) {} f();").is_err(),
        "parameter-scope eval declaring function arguments must throw"
    );
    // A *body*-scope direct eval shares the function var environment with
    // `arguments` and may redeclare it (sloppy `var arguments` in a plain
    // function body is allowed — Test262 language/statements/variable/12.2.1-11).
    assert!(
        eval("function f() { eval('var arguments = 1'); } f();").is_ok(),
        "body-scope eval declaring var arguments must be allowed"
    );
    // Reading `arguments` through eval is fine.
    assert_eq!(
        eval("function f() { return eval('arguments.length'); } f(1, 2, 3);"),
        Ok(Value::Number(3.0))
    );
    // Declaring a non-`arguments` var in a function eval is fine.
    assert_eq!(
        eval("function f() { return eval('var nonArg = 5; nonArg'); } f();"),
        Ok(Value::Number(5.0))
    );
    // Global eval may declare `var arguments` freely.
    assert_eq!(
        eval("eval('var arguments = 7'); arguments;"),
        Ok(Value::Number(7.0))
    );
}

#[test]
fn direct_eval_allows_arrow_body_arguments_binding() {
    assert_eq!(
        eval(
            "let f = (p = eval(\"var arguments = 'param'\")) => { \
             var arguments = 'local'; \
             return arguments; \
             }; \
             f();"
        ),
        Ok(Value::String("local".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let f = (p = eval(\"var arguments = 'param'\"), arguments) => {}; try { f(); } catch (error) { error instanceof SyntaxError; }"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let callCount = 0; \
             let f = (a = eval('var a = 42')) => { callCount = callCount + 1; }; \
             try { f(); } catch (error) { var caught = error instanceof SyntaxError; } \
             caught + ':' + callCount;"
        ),
        Ok(Value::String("true:0".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let f = () => { let local; eval('var local;'); }; \
             try { f(); } catch (error) { error instanceof SyntaxError; }"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn sloppy_parameter_eval_var_updates_parameter_closures() {
    assert_eq!(
        eval(
            "var scriptArgs = ['-e']; \
             var x = 'outside'; \
             var probe1, probe2, probeBody; \
             (function( \
               _ = (eval('var x = \"inside\";'), probe1 = function() { return x; }), \
               __ = probe2 = function() { return x; } \
             ) { \
               probeBody = function() { return x; }; \
             }()); \
             x + ':' + probe1() + ':' + probe2() + ':' + probeBody();"
        ),
        Ok(Value::String(
            "outside:inside:inside:inside".to_owned().into()
        ))
    );
    assert_eq!(
        eval(
            "var x = 'outside'; \
             var probe1, probe2; \
             (( \
               _ = probe1 = function() { return x; }, \
               ...[__ = (eval('var x = \"inside\";'), probe2 = function() { return x; })] \
             ) => {})(); \
             x + ':' + probe1() + ':' + probe2();"
        ),
        Ok(Value::String("outside:inside:inside".to_owned().into()))
    );
    assert_eq!(
        eval(
            "var x = 'outside'; \
             var probe1, probe2; \
             function* g( \
               _ = (eval('var x = \"inside\";'), probe1 = function() { return x; }), \
               __ = probe2 = function() { return x; } \
             ) {} \
             g().next(); \
             x + ':' + probe1() + ':' + probe2();"
        ),
        Ok(Value::String("outside:inside:inside".to_owned().into()))
    );
}

#[test]
fn direct_eval_rejects_return_even_inside_function() {
    assert_eq!(
        eval("try { eval('return;'); false; } catch (error) { error instanceof SyntaxError; }"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "function f() { \
               try { eval('return;'); return false; } \
               catch (error) { return error instanceof SyntaxError; } \
             } \
             f();"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn direct_eval_new_target_context_excludes_arrow_functions() {
    assert_eq!(
        eval("function f() { return eval('new.target === undefined'); } f();"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let f = () => eval('new.target;'); \
             try { f(); false; } catch (error) { error instanceof SyntaxError; }"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn sloppy_function_direct_eval_new_bindings_are_deletable() {
    assert_eq!(
        eval(
            "var initial, postDeletion; \
             (function() { \
               eval('initial = x; delete x; postDeletion = function() { x; }; var x;'); \
             }()); \
             (initial === undefined) + ':' + \
             (function() { try { postDeletion(); return false; } catch (error) { return error instanceof ReferenceError; } }());"
        ),
        Ok(Value::String("true:true".to_owned().into()))
    );
    assert_eq!(
        eval(
            "var initial, postDeletion; \
             (function() { \
               eval('initial = f; delete f; postDeletion = function() { f; }; function f() { return 33; }'); \
             }()); \
             (typeof initial) + ':' + initial() + ':' + \
             (function() { try { postDeletion(); return false; } catch (error) { return error instanceof ReferenceError; } }());"
        ),
        Ok(Value::String("function:33:true".to_owned().into()))
    );
}

#[test]
fn eval_script_runs_global_declaration_instantiation_checks() {
    // $262.evalScript performs GlobalDeclarationInstantiation: a var/function
    // declaration that cannot be created on a non-extensible global is a
    // TypeError thrown before any evaluation.
    assert!(
        eval("Object.preventExtensions(this); __quickjsRustEvalScript('var brandNewGlobalA;');")
            .is_err()
    );
    assert!(
        eval("Object.preventExtensions(this); __quickjsRustEvalScript('function brandNewGlobalB() {}');")
            .is_err()
    );
    // A declarable var still works on an extensible global.
    assert_eq!(
        eval("__quickjsRustEvalScript('var okGlobalVar = 7;'); okGlobalVar;"),
        Ok(Value::Number(7.0))
    );
    assert_eq!(
        eval(
            "__quickjsRustEvalScript('var evalScriptVar;'); \
             let d = Object.getOwnPropertyDescriptor(this, 'evalScriptVar'); \
             d.writable + ':' + d.enumerable + ':' + d.configurable;"
        ),
        Ok(Value::String("true:true:false".to_owned().into()))
    );
    assert_eq!(
        eval(
            "__quickjsRustEvalScript('function evalScriptFunction() {}'); \
             let d = Object.getOwnPropertyDescriptor(this, 'evalScriptFunction'); \
             (typeof evalScriptFunction) + ':' + d.writable + ':' + d.enumerable + ':' + d.configurable;"
        ),
        Ok(Value::String("function:true:true:false".to_owned().into()))
    );
    assert_eq!(
        eval(
            "Object.defineProperty(this, 'evalScriptConfigurableFunction', { value: 1, writable: false, enumerable: false, configurable: true }); \
             __quickjsRustEvalScript('function evalScriptConfigurableFunction() { return 2; }'); \
             let d = Object.getOwnPropertyDescriptor(this, 'evalScriptConfigurableFunction'); \
             evalScriptConfigurableFunction() + ':' + d.writable + ':' + d.enumerable + ':' + d.configurable;"
        ),
        Ok(Value::String("2:true:true:false".to_owned().into()))
    );
    assert_eq!(
        eval(
            "Object.defineProperty(this, 'evalScriptNonConfigurableFunction', { value: 1, writable: true, enumerable: true, configurable: false }); \
             __quickjsRustEvalScript('function evalScriptNonConfigurableFunction() { return 3; }'); \
             let d = Object.getOwnPropertyDescriptor(this, 'evalScriptNonConfigurableFunction'); \
             evalScriptNonConfigurableFunction() + ':' + d.writable + ':' + d.enumerable + ':' + d.configurable;"
        ),
        Ok(Value::String("3:true:true:false".to_owned().into()))
    );
    assert_eq!(
        eval(
            "__quickjsRustEvalScript('let evalScriptLexical;'); \
             try { __quickjsRustEvalScript('var createdBeforeLexicalRedecl; let evalScriptLexical;'); 'no throw'; } \
             catch (error) { error.name + ':' + this.hasOwnProperty('createdBeforeLexicalRedecl'); }"
        ),
        Ok(Value::String("SyntaxError:false".to_owned().into()))
    );
    assert_eq!(
        eval(
            "__quickjsRustEvalScript('const evalScriptConst = 1;'); \
             try { __quickjsRustEvalScript('var createdBeforeConstCollision; function evalScriptConst() {}'); 'no throw'; } \
             catch (error) { error.name + ':' + this.hasOwnProperty('createdBeforeConstCollision'); }"
        ),
        Ok(Value::String("SyntaxError:false".to_owned().into()))
    );
    assert_eq!(
        eval(
            "var test262Shim = { evalScript: function(source) { return __quickjsRustEvalScript(source); } }; \
             let shimLexical; \
             try { test262Shim.evalScript('var createdBeforeShimCollision; var shimLexical;'); 'no throw'; } \
             catch (error) { error.name + ':' + this.hasOwnProperty('createdBeforeShimCollision'); }"
        ),
        Ok(Value::String("SyntaxError:false".to_owned().into()))
    );
    assert_eq!(
        eval(
            "'use strict'; \
             { function strictBlockFunction() {} } \
             (typeof strictBlockFunction) + ':' + this.hasOwnProperty('strictBlockFunction');"
        ),
        Ok(Value::String("undefined:false".to_owned().into()))
    );
}

#[test]
fn test262_build_string_host_helper_matches_regexp_utils_shape() {
    assert_eq!(
        eval(
            "let value = __quickjsRustBuildString({ loneCodePoints: [65, 0x1F600], ranges: [[0x61, 0x63], [0xD800, 0xD800]] }); \
             value.length + ':' + value.charCodeAt(0) + ':' + value.codePointAt(1) + ':' + value.charCodeAt(3) + ':' + value.charCodeAt(5) + ':' + value.charCodeAt(6);"
        ),
        Ok(Value::String("7:65:128512:97:99:55296".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let calls = []; \
             let args = {}; \
             Object.defineProperty(args, 'loneCodePoints', { get: function() { calls.push('lone'); return [66]; } }); \
             Object.defineProperty(args, 'ranges', { get: function() { calls.push('ranges'); return [[67, 68]]; } }); \
             __quickjsRustBuildString(args) + ':' + calls.join(',');"
        ),
        Ok(Value::String("BCD:lone,ranges".to_owned().into()))
    );
}

#[test]
fn test262_verify_property_host_helper_checks_data_descriptors() {
    assert_eq!(
        eval(
            "let o = {}; \
             Object.defineProperty(o, 'x', { value: 7, writable: true, enumerable: true, configurable: true }); \
             __quickjsRustVerifyProperty(o, 'x', { value: 7, writable: true, enumerable: true, configurable: true });"
        ),
        Ok(Value::Boolean(true))
    );
    assert!(
        eval(
            "let o = {}; \
             Object.defineProperty(o, 'x', { value: 7, writable: true, enumerable: true, configurable: true }); \
             __quickjsRustVerifyProperty(o, 'x', { value: 8 });"
        )
        .is_err()
    );
    assert_eq!(
        eval(
            "let o = {}; \
             Object.defineProperty(o, 'x', { get: function() { return 1; }, configurable: true }); \
             __quickjsRustVerifyProperty(o, 'x', { get: o.__lookupGetter__('x'), configurable: true });"
        ),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval(
            "__quickjsRustVerifyProperty([1, 2], 'length', { value: 2, writable: true, enumerable: false, configurable: false });"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let o = { x: 1 }; \
             let desc = {}; \
             Object.defineProperty(desc, 'value', { get: function() { return 1; } }); \
             __quickjsRustVerifyProperty(o, 'x', desc);"
        ),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval(
            "let p = new Proxy({ x: 1 }, {}); \
             __quickjsRustVerifyProperty(p, 'x', { value: 1, writable: true, enumerable: true, configurable: true });"
        ),
        Ok(Value::Boolean(false))
    );
}
