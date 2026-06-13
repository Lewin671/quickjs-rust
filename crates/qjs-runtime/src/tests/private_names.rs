use crate::{Value, eval};

#[test]
fn private_async_generator_method_is_an_async_generator() {
    // A private `async *#m` must be wired as an async generator (its objects
    // carry the AsyncGenerator prototype), not a sync generator.
    assert_eq!(
        eval(
            "class C { async *#m() { yield 1; } get m() { return this.#m; } } \
             Object.prototype.toString.call(new C().m());"
        ),
        Ok(Value::String("[object AsyncGenerator]".to_owned()))
    );
    // A private sync `*#m` stays a sync generator.
    assert_eq!(
        eval(
            "class C { *#m() { yield 1; } get m() { return this.#m; } } \
             Object.prototype.toString.call(new C().m());"
        ),
        Ok(Value::String("[object Generator]".to_owned()))
    );
}

#[test]
fn reads_private_instance_field() {
    assert_eq!(
        eval("class C { #x = 5; getX() { return this.#x; } } new C().getX();"),
        Ok(Value::Number(5.0))
    );
}

#[test]
fn private_field_without_initializer_is_undefined() {
    assert_eq!(
        eval("class C { #x; getX() { return this.#x; } } new C().getX();"),
        Ok(Value::Undefined)
    );
}

#[test]
fn writes_private_instance_field() {
    assert_eq!(
        eval(
            "class C { #x = 1; set(v) { this.#x = v; } get() { return this.#x; } } let c = new C(); c.set(9); c.get();"
        ),
        Ok(Value::Number(9.0))
    );
}

#[test]
fn destructuring_assignment_writes_private_field_targets() {
    assert_eq!(
        eval(
            "class C {
               #x = 0;
               objectSet(source) { ({a: this.#x} = source); return this.#x; }
               arraySet(source) { [this.#x] = source; return this.#x; }
             }
             let c = new C();
             c.objectSet({a: 4}) + c.arraySet([5]);"
        ),
        Ok(Value::Number(9.0))
    );
}

#[test]
fn for_in_and_for_of_write_private_field_targets() {
    assert_eq!(
        eval(
            "class C {
               #x = '';
               forOf() { for (this.#x of [2]) {} return this.#x; }
               forIn() { for (this.#x in {a: 0}) {} return this.#x; }
             }
             let c = new C();
             c.forOf() + ':' + c.forIn();"
        ),
        Ok(Value::String("2:a".to_owned()))
    );
}

#[test]
fn private_field_destructuring_target_evaluates_before_source_get() {
    let result = eval(
        "class C extends class {} {
           #field;
           constructor() {
             let init = () => super();
             let object = { get a() { init(); } };
             ({a: this.#field} = object);
           }
         }
         new C();",
    );
    assert!(
        matches!(result, Err(ref error) if error.message.contains("ReferenceError")),
        "expected ReferenceError before source getter, got {result:?}"
    );
}

#[test]
fn duplicate_private_method_or_accessor_brand_is_type_error() {
    let method = eval(
        "class Base { constructor(o) { return o; } } \
         class C extends Base { #m() {} } \
         let obj = {}; new C(obj); new C(obj);",
    );
    assert!(
        matches!(method, Err(ref error) if error.message.contains("TypeError")),
        "expected duplicate private method brand TypeError, got {method:?}"
    );

    let accessor = eval(
        "class Base { constructor(o) { return o; } } \
         class C extends Base { get #p() { return 1; } set #p(v) {} } \
         let obj = {}; new C(obj); new C(obj);",
    );
    assert!(
        matches!(accessor, Err(ref error) if error.message.contains("TypeError")),
        "expected duplicate private accessor brand TypeError, got {accessor:?}"
    );
}

#[test]
fn private_getter_setter_pair_brands_once_per_instance() {
    assert_eq!(
        eval(
            "class C { get #p() { return 1; } set #p(v) {} read() { return this.#p; } } \
             new C().read();"
        ),
        Ok(Value::Number(1.0))
    );
}

#[test]
fn instance_public_and_private_fields_initialize_in_source_order() {
    assert_eq!(
        eval("class C { #x; y = this.#x; } new C().y;"),
        Ok(Value::Undefined)
    );
    assert_eq!(
        eval(
            "class C { #x; y = (this.#x = 1); getX() { return this.#x; } } let c = new C(); c.y + c.getX();"
        ),
        Ok(Value::Number(2.0))
    );

    let read_before_declaration = eval("class C { y = this.#x; #x; } new C();");
    assert!(
        matches!(read_before_declaration, Err(ref error) if error.message.contains("TypeError")),
        "expected TypeError for reading #x before its field is initialized, got {read_before_declaration:?}"
    );

    let write_before_declaration = eval("class C { y = (this.#x = 1); #x; } new C();");
    assert!(
        matches!(write_before_declaration, Err(ref error) if error.message.contains("TypeError")),
        "expected TypeError for writing #x before its field is initialized, got {write_before_declaration:?}"
    );
}

#[test]
fn instance_private_methods_brand_in_source_order() {
    assert_eq!(
        eval("class C { #m() { return 2; } y = this.#m(); } new C().y;"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval("class C { y = this.#m(); #m() { return 2; } } new C().y;"),
        Ok(Value::Number(2.0))
    );
}

#[test]
fn escaped_private_names_resolve_to_decoded_identity() {
    assert_eq!(
        eval(
            r"class C {
                #\u{6F} = 1;
                #\u2118() { return this.#\u{6F}; }
                set(value) { this.#\u{6F} = value; }
                get() { return this.#\u2118(); }
              }
              let c = new C();
              c.set(7);
              c.get();"
        ),
        Ok(Value::Number(7.0))
    );
}

#[test]
fn reads_static_private_field() {
    assert_eq!(
        eval("class C { static #s = 42; static getS() { return C.#s; } } C.getS();"),
        Ok(Value::Number(42.0))
    );
}

#[test]
fn calls_private_method() {
    assert_eq!(
        eval(
            "class C { #v = 3; #double() { return this.#v * 2; } run() { return this.#double(); } } new C().run();"
        ),
        Ok(Value::Number(6.0))
    );
}

#[test]
fn calls_static_private_method() {
    assert_eq!(
        eval(
            "class C { static #helper() { return 99; } static call() { return C.#helper(); } } C.call();"
        ),
        Ok(Value::Number(99.0))
    );
}

#[test]
fn private_getter_and_setter() {
    assert_eq!(
        eval(
            "class C { #x = 4; get #g() { return this.#x * 10; } set #g(v) { this.#x = v; } \
             read() { return this.#g; } write(v) { this.#g = v; } } \
             let c = new C(); let before = c.read(); c.write(7); before + c.read();"
        ),
        // before = 40, after write(7) read() = 70 → 110
        Ok(Value::Number(110.0))
    );
}

#[test]
fn private_elements_on_proxy_receiver_bypass_get_trap() {
    assert_eq!(
        eval(
            "let hits = []; \
             class Base { constructor() { return new Proxy(this, { get(obj, prop) { hits.push(prop); return obj[prop]; } }); } } \
             class C extends Base { #f = 3; method() { return this.#f; } } \
             let c = new C(); c.method() + ':' + hits.join('|');"
        ),
        Ok(Value::String("3:method".to_owned()))
    );
    assert_eq!(
        eval(
            "let hits = []; \
             class Base { constructor() { return new Proxy(this, { get(obj, prop) { hits.push(prop); return obj[prop]; } }); } } \
             class C extends Base { get #f() { return 3; } method() { return this.#f; } } \
             let c = new C(); c.method() + ':' + hits.join('|');"
        ),
        Ok(Value::String("3:method".to_owned()))
    );
    assert_eq!(
        eval(
            "let hits = []; \
             class Base { constructor() { return new Proxy(this, { get(obj, prop) { hits.push(prop); return obj[prop]; } }); } } \
             class C extends Base { #m() { return 3; } method() { return this.#m(); } } \
             let c = new C(); c.method() + ':' + hits.join('|');"
        ),
        Ok(Value::String("3:method".to_owned()))
    );
}

#[test]
fn foreign_object_access_throws_type_error() {
    let result = eval("class C { #x = 1; read(o) { return o.#x; } } new C().read({});");
    assert!(
        matches!(&result, Err(error) if error.message.contains("TypeError")
            && error.message.contains("#x")),
        "expected a TypeError about #x, got {result:?}"
    );
}

#[test]
fn brand_check_true_and_false() {
    assert_eq!(
        eval("class C { #x = 1; has(o) { return #x in o; } } let c = new C(); c.has(c);"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("class C { #x = 1; has(o) { return #x in o; } } new C().has({});"),
        Ok(Value::Boolean(false))
    );
}

#[test]
fn method_brand_check() {
    assert_eq!(
        eval("class C { #m() {} has(o) { return #m in o; } } let c = new C(); c.has(c);"),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn private_compound_assignment() {
    assert_eq!(
        eval("class C { #x = 10; bump() { this.#x += 5; return this.#x; } } new C().bump();"),
        Ok(Value::Number(15.0))
    );
}

#[test]
fn private_increment_postfix_and_prefix() {
    assert_eq!(
        eval(
            "class C { #x = 1; post() { return this.#x++; } get() { return this.#x; } } \
              let c = new C(); let p = c.post(); p * 100 + c.get();"
        ),
        // postfix returns 1, then #x becomes 2 → 1*100 + 2 = 102
        Ok(Value::Number(102.0))
    );
    assert_eq!(
        eval("class C { #x = 1; pre() { return ++this.#x; } } new C().pre();"),
        Ok(Value::Number(2.0))
    );
}

#[test]
fn sibling_instances_have_independent_fields() {
    assert_eq!(
        eval(
            "class C { #x = 0; set(v) { this.#x = v; } get() { return this.#x; } } \
             let a = new C(); let b = new C(); a.set(5); b.set(9); a.get() * 100 + b.get();"
        ),
        Ok(Value::Number(509.0))
    );
}

#[test]
fn fresh_identity_per_class_evaluation() {
    // Two evaluations of the same class expression mint distinct private-name
    // identities, so an instance of one is not branded for the other.
    assert_eq!(
        eval(
            "function mk() { return class { #x = 1; has(o) { return #x in o; } }; } \
             let A = mk(); let B = mk(); let a = new A(); new B().has(a);"
        ),
        Ok(Value::Boolean(false))
    );
    // ...and reading across evaluations throws.
    let result = eval(
        "function mk() { return class { #x = 1; read(o) { return o.#x; } }; } \
         let A = mk(); let B = mk(); let a = new A(); new B().read(a);",
    );
    assert!(
        matches!(&result, Err(error) if error.message.contains("TypeError")),
        "expected a TypeError across evaluations, got {result:?}"
    );
}

#[test]
fn nested_class_resolves_outer_private_name() {
    assert_eq!(
        eval(
            "class Outer { #o = 7; make() { const self = this; \
             return class Inner { read() { return self.#o; } }; } } \
             let o = new Outer(); let I = o.make(); new I().read();"
        ),
        Ok(Value::Number(7.0))
    );
}

#[test]
fn private_name_is_visible_to_computed_class_key() {
    let result = eval(
        "const self = this; \
         class C { [self.#f] = 'Test262'; #f = 'foo'; }",
    );
    assert!(
        matches!(&result, Err(error) if error.message.contains("TypeError")
            && error.message.contains("#f")),
        "expected a TypeError for visible private name on unbranded object, got {result:?}"
    );
}

#[test]
fn direct_eval_in_field_initializer_resolves_private_field() {
    assert_eq!(
        eval("class C { #m = 44; v = eval('this.#m'); } new C().v;"),
        Ok(Value::Number(44.0))
    );
}

#[test]
fn nested_function_resolves_enclosing_private_field() {
    assert_eq!(
        eval(
            "class C { #f = 'ok'; read() { let self = this; \
             function inner() { return self.#f; } return inner(); } } new C().read();"
        ),
        Ok(Value::String("ok".to_owned()))
    );
}

#[test]
fn nested_arrow_resolves_enclosing_private_method() {
    assert_eq!(
        eval(
            "class C { #m() { return 11; } read() { const inner = () => this.#m(); \
             return inner(); } } new C().read();"
        ),
        Ok(Value::Number(11.0))
    );
}

#[test]
fn nested_function_resolves_static_private_method() {
    assert_eq!(
        eval(
            "class C { static #m() { return 23; } static read() { \
             function inner() { return C.#m(); } return inner(); } } C.read();"
        ),
        Ok(Value::Number(23.0))
    );
}

#[test]
fn derived_class_instance_private_field() {
    assert_eq!(
        eval("class A {} class B extends A { #x = 3; getX() { return this.#x; } } new B().getX();"),
        Ok(Value::Number(3.0))
    );
}

#[test]
fn private_method_is_not_writable() {
    let result = eval("class C { #m() {} go() { this.#m = 1; } } new C().go();");
    assert!(
        matches!(&result, Err(error) if error.message.contains("TypeError")),
        "writing a private method should throw, got {result:?}"
    );
}
