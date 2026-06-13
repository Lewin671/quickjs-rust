use crate::{Value, eval};

#[test]
fn instantiates_class_and_reads_field() {
    assert_eq!(
        eval("class C { constructor(x) { this.x = x; } } new C(7).x;"),
        Ok(Value::Number(7.0))
    );
}

#[test]
fn static_initialization_block_runs_with_this_as_constructor() {
    assert_eq!(
        eval("class C { static { this.x = 7; } } C.x;"),
        Ok(Value::Number(7.0))
    );
}

#[test]
fn static_blocks_and_fields_run_in_source_order() {
    // Each block/field appends to a shared array; the order is field, block,
    // field, block.
    assert_eq!(
        eval(
            "class C { \
               static log = []; \
               static a = C.log.push(1); \
               static { C.log.push(2); } \
               static b = C.log.push(3); \
               static { C.log.push(4); } \
             } C.log.join(',');"
        ),
        Ok(Value::String("1,2,3,4".to_owned()))
    );
}

#[test]
fn static_block_can_read_super_property() {
    assert_eq!(
        eval(
            "class B { static base = 10; } \
             class C extends B { static { this.v = super.base + 1; } } C.v;"
        ),
        Ok(Value::Number(11.0))
    );
}

#[test]
fn numeric_literal_member_keys_use_their_canonical_name() {
    // A numeric-literal method/accessor key names the property `ToString(MV)`,
    // so `0b10` defines `"2"` (matching object literals).
    assert_eq!(
        eval("class C { get 0b10() { return 'g'; } } C.prototype['2'];"),
        Ok(Value::String("g".to_owned()))
    );
    assert_eq!(
        eval("class C { 0x10() { return 5; } } new C()['16']();"),
        Ok(Value::Number(5.0))
    );
    assert_eq!(
        eval(
            "class C { \
               [1_2_3]() { return 1_2_3; } \
               static [0x1_0]() { return 0x1_0; } \
             } \
             let c = new C(); c[123]() + C[16]();"
        ),
        Ok(Value::Number(139.0))
    );
}

#[test]
fn calls_prototype_method() {
    assert_eq!(
        eval(
            "class C { constructor(x) { this.x = x; } twice() { return this.x * 2; } } new C(21).twice();"
        ),
        Ok(Value::Number(42.0))
    );
}

#[test]
fn method_this_binds_to_instance() {
    assert_eq!(
        eval(
            "class Counter { constructor() { this.n = 0; } inc() { this.n += 1; return this.n; } } let c = new Counter(); c.inc(); c.inc();"
        ),
        Ok(Value::Number(2.0))
    );
}

#[test]
fn default_constructor_creates_instance() {
    assert_eq!(
        eval("class C {} typeof new C();"),
        Ok(Value::String("object".to_owned()))
    );
}

#[test]
fn default_derived_constructor_arguments_do_not_shadow_outer_bindings() {
    assert_eq!(
        eval(
            "var args, that; \
             class Base { constructor() { that = this; args = arguments; } } \
             class Derived extends Base {} \
             new Derived(0, 1, 2); \
             args.length + ':' + (that instanceof Derived);"
        ),
        Ok(Value::String("3:true".to_owned()))
    );
}

#[test]
fn class_method_computed_object_binding_key_propagates_errors() {
    assert!(
        eval(
            "function thrower() { throw new Error('key'); } \
             class C { method({ [thrower()]: x }) {} } \
             new C().method({});"
        )
        .is_err(),
        "computed binding property key errors must propagate from parameter binding"
    );
}

#[test]
fn class_method_computed_object_binding_key_is_evaluated_once() {
    assert_eq!(
        eval(
            "var calls = 0; \
             var key = { toString() { calls += 1; return 'x'; } }; \
             class C { method({ [key]: value }) { return value + ':' + calls; } } \
             new C().method({ x: 7 });"
        ),
        Ok(Value::String("7:1".to_owned()))
    );
}

#[test]
fn class_method_computed_object_binding_key_is_excluded_from_rest() {
    assert_eq!(
        eval(
            "var key = 'x'; \
             class C { method({ [key]: value, ...rest }) { return value + ':' + rest.x + ':' + rest.y; } } \
             new C().method({ x: 1, y: 2 });"
        ),
        Ok(Value::String("1:undefined:2".to_owned()))
    );
}

#[test]
fn explicit_derived_constructor_must_call_super_before_returning() {
    assert!(
        eval("class B {} class C extends B { constructor() {} } new C();").is_err(),
        "derived constructor without super() must throw"
    );
}

#[test]
fn derived_super_property_requires_initialized_this() {
    assert!(
        eval(
            "class B {} \
             class C extends B { constructor() { super.m(); } } \
             new C();"
        )
        .is_err(),
        "super property access before super() must throw"
    );
}

#[test]
fn repeated_super_call_runs_parent_before_reference_error() {
    assert_eq!(
        eval(
            "var calls = 0; \
             class B { constructor() { calls += 1; } } \
             class C extends B { \
               constructor() { \
                 super(); \
                 try { super(); } catch (e) {} \
               } \
             } \
             new C(); calls;"
        ),
        Ok(Value::Number(2.0))
    );
}

#[test]
fn instance_is_instanceof_class() {
    assert_eq!(
        eval("class C {} (new C()) instanceof C;"),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn prototype_constructor_back_reference() {
    assert_eq!(
        eval("class C {} C.prototype.constructor === C;"),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn prototype_property_is_non_writable_non_enumerable_non_configurable() {
    assert_eq!(
        eval(
            "class C {} let d = Object.getOwnPropertyDescriptor(C, 'prototype'); [d.writable, d.enumerable, d.configurable].join(',');"
        ),
        Ok(Value::String("false,false,false".to_owned()))
    );
}

#[test]
fn constructor_back_reference_is_writable_non_enumerable_configurable() {
    assert_eq!(
        eval(
            "class C {} let d = Object.getOwnPropertyDescriptor(C.prototype, 'constructor'); [d.writable, d.enumerable, d.configurable].join(',');"
        ),
        Ok(Value::String("true,false,true".to_owned()))
    );
}

#[test]
fn methods_are_non_enumerable_writable_configurable() {
    assert_eq!(
        eval(
            "class C { m() {} } let d = Object.getOwnPropertyDescriptor(C.prototype, 'm'); [d.writable, d.enumerable, d.configurable].join(',');"
        ),
        Ok(Value::String("true,false,true".to_owned()))
    );
}

#[test]
fn methods_are_not_own_enumerable_keys() {
    assert_eq!(
        eval("class C { a() {} b() {} } Object.keys(C.prototype).length;"),
        Ok(Value::Number(0.0))
    );
}

#[test]
fn constructor_name_comes_from_binding() {
    assert_eq!(
        eval("class C {} C.name;"),
        Ok(Value::String("C".to_owned()))
    );
}

#[test]
fn constructor_length_comes_from_constructor_params() {
    assert_eq!(
        eval("class C { constructor(a, b, c) {} } C.length;"),
        Ok(Value::Number(3.0))
    );
}

#[test]
fn default_constructor_has_zero_length() {
    assert_eq!(eval("class C {} C.length;"), Ok(Value::Number(0.0)));
}

#[test]
fn calling_class_without_new_throws_type_error() {
    assert_eq!(
        eval(
            "class C {} try { C(); 'no throw'; } catch (e) { e instanceof TypeError ? 'TypeError' : 'other'; }"
        ),
        Ok(Value::String("TypeError".to_owned()))
    );
}

#[test]
fn derived_constructor_returning_symbol_throws_type_error() {
    // A Symbol (a primitive, not an Object) returned from a derived constructor
    // does not override `this`; it is a TypeError like any other primitive.
    assert_eq!(
        eval(
            "class B {} class D extends B { constructor() { super(); return Symbol(); } } \
             try { new D(); 'no throw'; } catch (e) { e instanceof TypeError ? 'TypeError' : 'other'; }"
        ),
        Ok(Value::String("TypeError".to_owned()))
    );
    // An object return still overrides `this`.
    assert_eq!(
        eval(
            "class B {} class D extends B { constructor() { super(); return { tag: 9 }; } } \
             new D().tag;"
        ),
        Ok(Value::Number(9.0))
    );
}

#[test]
fn method_is_not_constructable() {
    assert_eq!(
        eval(
            "class C { m() {} } let c = new C(); try { new c.m(); 'no throw'; } catch (e) { e instanceof TypeError ? 'TypeError' : 'other'; }"
        ),
        Ok(Value::String("TypeError".to_owned()))
    );
}

#[test]
fn method_has_no_prototype_property() {
    assert_eq!(
        eval("class C { m() {} } C.prototype.m.prototype === undefined;"),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn class_expression_anonymous_is_constructable() {
    assert_eq!(
        eval("let D = class { value() { return 9; } }; new D().value();"),
        Ok(Value::Number(9.0))
    );
}

#[test]
fn named_class_expression_exposes_name() {
    assert_eq!(
        eval("let x = class Named {}; x.name;"),
        Ok(Value::String("Named".to_owned()))
    );
}

#[test]
fn named_class_expression_name_is_visible_inside_methods() {
    assert_eq!(
        eval("let x = class Named { self() { return Named; } }; let i = new x(); i.self() === x;"),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn class_inner_name_binding_is_immutable_inside_members() {
    assert_eq!(
        eval(
            "let checks = [
                () => { class C { constructor() { C = 42; } }; new C(); },
                () => { class C { m() { C = 42; } }; new C().m(); },
                () => { class C { get x() { C = 42; } }; new C().x; },
                () => { class C { set x(v) { C = 42; } }; new C().x = 1; },
                () => { class C { x = (C = 42); }; new C(); },
                () => { class C { static { C = 42; } }; }
             ];
             checks.every((fn) => {
                try { fn(); return false; }
                catch (e) { return e instanceof TypeError; }
             });",
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn class_inner_name_binding_can_be_shadowed_by_parameters() {
    assert_eq!(
        eval("class C { m(C) { C = 42; return C; } } new C().m(1);"),
        Ok(Value::Number(42.0))
    );
}

#[test]
fn class_declaration_is_block_scoped() {
    assert_eq!(
        eval("{ class C {} } typeof C;"),
        Ok(Value::String("undefined".to_owned()))
    );
}

#[test]
fn class_declaration_is_not_hoisted_like_var() {
    // Class declarations create lexical bindings, not var-style hoisted ones,
    // so referencing the class before its declaration is a ReferenceError.
    assert_eq!(
        eval(
            "try { let probe = C; 'no throw'; class C {} } catch (e) { e instanceof ReferenceError ? 'ReferenceError' : 'other'; }"
        ),
        Ok(Value::String("ReferenceError".to_owned()))
    );
}

#[test]
fn class_body_is_strict_mode_code() {
    // Assigning to an undeclared name inside a method throws in strict mode.
    assert_eq!(
        eval(
            "class C { m() { undeclaredStrict = 1; } } try { new C().m(); 'no throw'; } catch (e) { e instanceof ReferenceError ? 'ReferenceError' : 'other'; }"
        ),
        Ok(Value::String("ReferenceError".to_owned()))
    );
}

#[test]
fn method_closes_over_enclosing_scope() {
    assert_eq!(
        eval("let base = 100; class C { m() { return base + 1; } } new C().m();"),
        Ok(Value::Number(101.0))
    );
}

#[test]
fn static_method_installs_on_constructor() {
    assert_eq!(
        eval("class C { static make() { return 7; } } C.make();"),
        Ok(Value::Number(7.0))
    );
}

#[test]
fn static_method_is_not_on_prototype() {
    assert_eq!(
        eval("class C { static m() {} } typeof C.prototype.m;"),
        Ok(Value::String("undefined".to_owned()))
    );
}

#[test]
fn static_method_descriptor_is_non_enumerable_writable_configurable() {
    assert_eq!(
        eval(
            "class C { static m() {} } let d = Object.getOwnPropertyDescriptor(C, 'm'); [d.writable, d.enumerable, d.configurable].join(',');"
        ),
        Ok(Value::String("true,false,true".to_owned()))
    );
}

#[test]
fn static_accessor_roundtrips() {
    assert_eq!(
        eval(
            "class C { static get c() { return C._c; } static set c(v) { C._c = v; } } C.c = 11; C.c;"
        ),
        Ok(Value::Number(11.0))
    );
}

#[test]
fn instance_getter_reads_state() {
    assert_eq!(
        eval("class C { constructor() { this._v = 4; } get v() { return this._v; } } new C().v;"),
        Ok(Value::Number(4.0))
    );
}

#[test]
fn instance_setter_writes_state() {
    assert_eq!(
        eval(
            "class C { set v(x) { this._v = x * 2; } get v() { return this._v; } } let c = new C(); c.v = 5; c.v;"
        ),
        Ok(Value::Number(10.0))
    );
}

#[test]
fn getter_and_setter_merge_into_one_accessor() {
    assert_eq!(
        eval(
            "class C { get x() { return this._x; } set x(v) { this._x = v; } } let d = Object.getOwnPropertyDescriptor(C.prototype, 'x'); [typeof d.get, typeof d.set].join(',');"
        ),
        Ok(Value::String("function,function".to_owned()))
    );
}

#[test]
fn accessor_descriptor_is_non_enumerable_configurable() {
    assert_eq!(
        eval(
            "class C { get x() {} } let d = Object.getOwnPropertyDescriptor(C.prototype, 'x'); [d.enumerable, d.configurable].join(',');"
        ),
        Ok(Value::String("false,true".to_owned()))
    );
}

#[test]
fn accessor_is_not_own_enumerable_key() {
    assert_eq!(
        eval("class C { get x() {} } Object.keys(C.prototype).length;"),
        Ok(Value::Number(0.0))
    );
}

#[test]
fn computed_method_name_resolves_at_definition() {
    assert_eq!(
        eval("let k = 'foo'; class C { [k]() { return 9; } } new C().foo();"),
        Ok(Value::Number(9.0))
    );
}

#[test]
fn static_computed_method_name() {
    assert_eq!(
        eval("let k = 'm'; class C { static [k]() { return 5; } } C.m();"),
        Ok(Value::Number(5.0))
    );
}

#[test]
fn computed_keys_evaluate_in_source_order() {
    assert_eq!(
        eval(
            "let log = []; function k(n) { log.push(n); return 'm' + n; } class C { [k(1)]() {} [k(2)]() {} } log.join(',');"
        ),
        Ok(Value::String("1,2".to_owned()))
    );
}

#[test]
fn symbol_computed_method_name() {
    assert_eq!(
        eval("let s = Symbol('s'); class C { [s]() { return 42; } } new C()[s]();"),
        Ok(Value::Number(42.0))
    );
}

#[test]
fn static_get_set_are_valid_method_names() {
    assert_eq!(
        eval(
            "class C { static() { return 1; } get() { return 2; } set() { return 3; } } let c = new C(); [c.static(), c.get(), c.set()].join(',');"
        ),
        Ok(Value::String("1,2,3".to_owned()))
    );
}

#[test]
fn static_keyword_as_static_method() {
    assert_eq!(
        eval("class C { static static() { return 8; } } C.static();"),
        Ok(Value::Number(8.0))
    );
}

#[test]
fn subclass_instance_is_instanceof_both() {
    assert_eq!(
        eval(
            "class A {} class B extends A {} let b = new B(); [b instanceof B, b instanceof A].join(',');"
        ),
        Ok(Value::String("true,true".to_owned()))
    );
}

#[test]
fn subclass_prototype_chain_links_to_parent() {
    assert_eq!(
        eval("class A {} class B extends A {} Object.getPrototypeOf(B.prototype) === A.prototype;"),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn subclass_inherits_parent_method() {
    assert_eq!(
        eval("class A { hi() { return 'a'; } } class B extends A {} new B().hi();"),
        Ok(Value::String("a".to_owned()))
    );
}

#[test]
fn override_delegates_to_super_method() {
    assert_eq!(
        eval(
            "class A { who() { return 'A'; } } class B extends A { who() { return super.who() + 'B'; } } new B().who();"
        ),
        Ok(Value::String("AB".to_owned()))
    );
}

#[test]
fn super_call_binds_this_and_forwards_arguments() {
    assert_eq!(
        eval(
            "class A { constructor(x) { this.x = x; } } class B extends A { constructor(x) { super(x * 2); this.y = x; } } let b = new B(5); [b.x, b.y].join(',');"
        ),
        Ok(Value::String("10,5".to_owned()))
    );
}

#[test]
fn default_derived_constructor_forwards_arguments() {
    assert_eq!(
        eval(
            "class A { constructor(a, b) { this.sum = a + b; } } class B extends A {} new B(2, 5).sum;"
        ),
        Ok(Value::Number(7.0))
    );
}

#[test]
fn super_property_read_uses_parent_prototype() {
    assert_eq!(
        eval(
            "class A { val() { return 1; } } class B extends A { val() { return 2; } use() { return super.val() + this.val(); } } new B().use();"
        ),
        Ok(Value::Number(3.0))
    );
}

#[test]
fn super_computed_property_read() {
    assert_eq!(
        eval(
            "class A { m() { return 7; } } class B extends A { go() { let k = 'm'; return super[k](); } } new B().go();"
        ),
        Ok(Value::Number(7.0))
    );
}

#[test]
fn super_accessor_delegates_to_parent_getter() {
    assert_eq!(
        eval(
            "class A { get v() { return 10; } } class B extends A { get v() { return super.v + 5; } } new B().v;"
        ),
        Ok(Value::Number(15.0))
    );
}

#[test]
fn super_in_static_method_calls_parent_static() {
    assert_eq!(
        eval(
            "class A { static who() { return 'A'; } } class B extends A { static who() { return super.who() + 'B'; } } B.who();"
        ),
        Ok(Value::String("AB".to_owned()))
    );
}

#[test]
fn super_property_in_async_method_default_parameter() {
    let value = eval(
        "let log = []; \
         class A { async method() { return 'sup'; } } \
         class B extends A { \
           async method(x = super.method()) { log.push(await x); } \
         } \
         new B().method().then(() => log.push('done')); \
         log;",
    )
    .expect("async class method should evaluate");
    let Value::Array(array) = value else {
        panic!("expected log array");
    };
    assert_eq!(
        array.to_vec(),
        vec![
            Value::String("sup".to_owned()),
            Value::String("done".to_owned())
        ]
    );
}

#[test]
fn class_method_default_parameters_use_parameter_tdz() {
    assert_eq!(
        eval(
            "let calls = 0; class C { method(x = x) { calls = calls + 1; } } \
             let name; try { C.prototype.method(); } catch (error) { name = error.name; } \
             name + ':' + calls;"
        ),
        Ok(Value::String("ReferenceError:0".to_owned()))
    );
    assert_eq!(
        eval(
            "let calls = 0; class C { method(x = y, y) { calls = calls + 1; } } \
             let name; try { C.prototype.method(); } catch (error) { name = error.name; } \
             name + ':' + calls;"
        ),
        Ok(Value::String("ReferenceError:0".to_owned()))
    );
}

#[test]
fn named_class_expression_heritage_uses_inner_tdz_binding() {
    let result = eval("var x = (class x extends x {});");
    assert!(
        matches!(&result, Err(error) if error.message.contains("ReferenceError")),
        "expected ReferenceError, got {result:?}"
    );
}

#[test]
fn named_class_expression_inner_binding_is_visible_to_methods() {
    assert_eq!(
        eval("let C = class Inner { static self() { return Inner; } }; C.self() === C;"),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn subclass_inherits_static_method() {
    assert_eq!(
        eval("class A { static who() { return 'A'; } } class B extends A {} B.who();"),
        Ok(Value::String("A".to_owned()))
    );
}

#[test]
fn subclass_constructor_prototype_is_superclass() {
    // Static inheritance uses real [[Prototype]] identity, not a snapshot.
    assert_eq!(
        eval("class A {} class B extends A {} Object.getPrototypeOf(B) === A;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("class A {} class B extends A {} A.isPrototypeOf(B);"),
        Ok(Value::Boolean(true))
    );
    // A base class constructor's [[Prototype]] remains %Function.prototype%.
    assert_eq!(
        eval("class A {} Object.getPrototypeOf(A) === Function.prototype;"),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn subclass_constructor_restricted_function_properties_throw_on_write() {
    assert_eq!(
        eval(
            "class A {} class B extends A {} \
             let caller = false; let args = false; \
             try { B.caller = 1; } catch (error) { caller = error instanceof TypeError; } \
             try { B.arguments = 1; } catch (error) { args = error instanceof TypeError; } \
             [B.hasOwnProperty('caller'), B.hasOwnProperty('arguments'), caller, args].join(':');"
        ),
        Ok(Value::String("false:false:true:true".to_owned()))
    );
}

#[test]
fn subclass_static_inheritance_is_live() {
    // A static member added to the parent after subclassing is visible, proving
    // the link is by reference rather than a definition-time copy.
    assert_eq!(
        eval("class A {} class B extends A {} A.added = 9; B.added;"),
        Ok(Value::Number(9.0))
    );
}

#[test]
fn subclass_inherits_static_field() {
    assert_eq!(
        eval("class A { static x = 99; } class B extends A {} B.x;"),
        Ok(Value::Number(99.0))
    );
}

#[test]
fn static_super_resolves_through_function_prototype() {
    assert_eq!(
        eval(
            "class A { static who() { return 'super'; } } class B extends A { static who() { return 'sub-' + super.who(); } } B.who();"
        ),
        Ok(Value::String("sub-super".to_owned()))
    );
}

#[test]
fn this_before_super_is_reference_error() {
    let result =
        eval("class A {} class B extends A { constructor() { this.x = 1; super(); } } new B();");
    assert!(
        matches!(&result, Err(error) if error.message.contains("ReferenceError")),
        "expected ReferenceError, got {result:?}"
    );
}

#[test]
fn super_called_twice_is_reference_error() {
    let result =
        eval("class A {} class B extends A { constructor() { super(); super(); } } new B();");
    assert!(
        matches!(&result, Err(error) if error.message.contains("ReferenceError")),
        "expected ReferenceError, got {result:?}"
    );
}

#[test]
fn extends_non_constructor_is_type_error() {
    let result = eval("try { new (class extends 5 {})(); 'no-throw'; } catch (e) { '' + e; }");
    assert!(
        matches!(&result, Ok(Value::String(message)) if message.contains("TypeError")),
        "expected caught TypeError, got {result:?}"
    );
}

#[test]
fn extends_null_keeps_constructor_callable_and_null_prototype() {
    assert_eq!(
        eval("class C extends null {} typeof C;"),
        Ok(Value::String("function".to_owned()))
    );
    assert_eq!(
        eval("class C extends null {} Object.getPrototypeOf(C.prototype) === null;"),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn superclass_html_dda_prototype_is_an_object_for_heritage() {
    assert_eq!(
        eval(
            "function Superclass() {} \
             Superclass.prototype = __quickjsRustIsHTMLDDA; \
             class C extends Superclass {} \
             let c = new C(); \
             (c instanceof C) + ':' + (c instanceof Superclass);"
        ),
        Ok(Value::String("true:true".to_owned()))
    );
}

#[test]
fn new_target_propagates_through_super_for_prototype() {
    // A subclass instance built via `new B` has `B.prototype` on its chain
    // even though only the base class constructor allocates `this`.
    assert_eq!(
        eval(
            "class A { constructor() {} } class B extends A {} Object.getPrototypeOf(new B()) === B.prototype;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn derived_methods_are_not_constructable() {
    let result = eval("class A {} class B extends A { m() {} } new (new B().m)();");
    assert!(result.is_err(), "class methods must not be constructable");
}

#[test]
fn instance_field_with_initializer() {
    assert_eq!(
        eval("class C { x = 41; } new C().x;"),
        Ok(Value::Number(41.0))
    );
}

#[test]
fn instance_field_without_initializer_is_undefined() {
    assert_eq!(eval("class C { x; } new C().x;"), Ok(Value::Undefined));
}

#[test]
fn fields_initialize_in_definition_order_seeing_earlier_fields() {
    // A later field's initializer observes earlier fields through `this`.
    assert_eq!(
        eval("class C { a = 1; b = this.a + 1; c = this.b + 1; } new C().c;"),
        Ok(Value::Number(3.0))
    );
}

#[test]
fn field_shadows_prototype_method_and_is_own_enumerable() {
    assert_eq!(
        eval(
            "class C { m() { return 'method'; } m = 7; } let c = new C(); [typeof c.m, c.m, Object.keys(c).join(',')].join('|');"
        ),
        Ok(Value::String("number|7|m".to_owned()))
    );
}

#[test]
fn prototype_methods_stay_non_enumerable_with_fields() {
    assert_eq!(
        eval(
            "class C { x = 1; m() {} } Object.keys(C.prototype).length === 0 && Object.keys(new C()).join(',');"
        ),
        Ok(Value::String("x".to_owned()))
    );
}

#[test]
fn static_field_runs_with_this_as_constructor() {
    assert_eq!(
        eval("class C { static self = this; } C.self === C;"),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn static_field_value_installed_on_constructor() {
    assert_eq!(
        eval("class C { static n = 9; } C.n;"),
        Ok(Value::Number(9.0))
    );
}

#[test]
fn computed_field_key_evaluated_once_at_definition() {
    assert_eq!(
        eval(
            "var calls = 0; function k() { calls++; return 'f'; } class C { [k()] = 1; } new C(); new C(); calls;"
        ),
        Ok(Value::Number(1.0))
    );
}

#[test]
fn computed_field_key_installs_field() {
    assert_eq!(
        eval("let key = 'dyn'; class C { [key] = 5; } new C().dyn;"),
        Ok(Value::Number(5.0))
    );
}

#[test]
fn computed_static_and_instance_fields_preserve_evaluation_order() {
    assert_eq!(
        eval(
            "let i = 0; \
             class C { [i++] = i++; static [i++] = i++; [i++] = i++; } \
             let c = new C(); \
             [i, c[0], c[2], C[1], c.hasOwnProperty('1'), C.hasOwnProperty('0'), C.hasOwnProperty('2')].join(',');"
        ),
        Ok(Value::String("6,4,5,3,false,false,false".to_owned()))
    );
}

#[test]
fn derived_instance_fields_run_after_super() {
    // The field initializer sees state established by the base constructor.
    assert_eq!(
        eval(
            "class A { constructor() { this.tag = 'A'; } } class B extends A { x = this.tag + 'B'; } new B().x;"
        ),
        Ok(Value::String("AB".to_owned()))
    );
}

#[test]
fn default_derived_constructor_runs_field_init() {
    assert_eq!(
        eval(
            "class A { constructor() { this.a = 1; } } class B extends A { x = 5; } let b = new B(); b.a + b.x;"
        ),
        Ok(Value::Number(6.0))
    );
}

#[test]
fn default_base_constructor_runs_field_init() {
    assert_eq!(
        eval("class C { x = 3; } new C().x;"),
        Ok(Value::Number(3.0))
    );
}

#[test]
fn field_initializer_closes_over_class_scope() {
    assert_eq!(
        eval("let v = 10; class C { x = v; } new C().x;"),
        Ok(Value::Number(10.0))
    );
}

#[test]
fn super_property_in_field_initializer() {
    assert_eq!(
        eval("class A { get y() { return 8; } } class B extends A { x = super.y; } new B().x;"),
        Ok(Value::Number(8.0))
    );
}

#[test]
fn super_property_assignment_in_field_initializer_arrow() {
    assert_eq!(
        eval(
            "class C {
               func = () => { super.prop = 'test262'; };
               static staticFunc = () => { super.staticProp = 'static test262'; };
             }
             let c = new C();
             c.func();
             C.staticFunc();
             c.prop + ':' + C.staticProp;"
        ),
        Ok(Value::String("test262:static test262".to_owned()))
    );
}

#[test]
fn instance_fields_are_writable_and_configurable() {
    assert_eq!(
        eval(
            "class C { x = 1; } let c = new C(); let d = Object.getOwnPropertyDescriptor(c, 'x'); [d.writable, d.enumerable, d.configurable].join(',');"
        ),
        Ok(Value::String("true,true,true".to_owned()))
    );
}
