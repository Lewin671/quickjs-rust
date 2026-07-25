use crate::{Value, eval};

#[test]
fn class_members_write_enclosing_bindings() {
    assert_eq!(
        eval(
            "let count = 0; class C { m() { count++; } } \
             function run(fn) { fn(); } run(() => { new C().m(); }); count;"
        ),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("let count = 0; class C { constructor() { count++; } } new C(); count;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("let count = 0; class C { [count++]() {} } count;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("let count = 0; class C { static { count++; } } count;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval(
            "let count = 0; class C { #m() { count++; } run() { this.#m(); } } new C().run(); count;"
        ),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("let count = 0; class C { #x = count++; } new C(); count;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval(
            "function Test262Error(message) { \
               if (!(this instanceof Test262Error)) return new Test262Error(message); \
               this.message = message || ''; \
             } \
             let count = 0; \
             class C { #p = 1; method() { count++; try { count++; this.#p; } \
               catch (e) { count++; if (e instanceof TypeError) throw new Test262Error(); } } } \
             try { new C().method.call(15); } catch (e) {} count;"
        ),
        Ok(Value::Number(3.0))
    );
}

#[test]
fn class_members_keep_inner_name_binding_after_outer_mutation() {
    assert_eq!(
        eval(
            "class C { method() { return C; } } \
             var cls = C; \
             C = null; \
             [C === null, cls.prototype.method() === cls].join(':');"
        ),
        Ok(Value::String("true:true".to_owned().into()))
    );
    assert_eq!(
        eval(
            "var probeBefore = function() { return C; }; \
             var setBefore = function() { C = null; }; \
             class C { \
               probe() { return C; } \
               modify() { C = null; } \
             } \
             var cls = probeBefore(); \
             setBefore(); \
             var modifyThrows = false; \
             try { cls.prototype.modify(); } catch (e) { modifyThrows = e instanceof TypeError; } \
             [probeBefore() === null, cls.prototype.probe() === cls, modifyThrows, typeof cls.prototype.probe()].join(':');"
        ),
        Ok(Value::String("true:true:true:function".to_owned().into()))
    );
    assert_eq!(
        eval(
            "var setBefore = function() { C = null; }; \
             var probeBefore = function() { return C; }; \
             var probeHeritage, setHeritage; \
             class C extends ( \
               probeHeritage = function() { return C; }, \
               setHeritage = function() { C = null; } \
             ) { \
               method() { return C; } \
             } \
             var cls = probeBefore(); \
             setBefore(); \
             var heritageSetThrows = false; \
             try { setHeritage(); } catch (e) { heritageSetThrows = e instanceof TypeError; } \
             [probeBefore() === null, probeHeritage() === cls, heritageSetThrows, cls.prototype.method() === cls].join(':');"
        ),
        Ok(Value::String("true:true:true:true".to_owned().into()))
    );
}

#[test]
fn class_inner_name_stays_visible_to_every_member_that_uses_it() {
    // A class body's inner name is an immutable binding in scope for every
    // member. Members that never mention it no longer carry the binding, so
    // every route by which a member can reach it must still work.
    assert_eq!(
        eval(
            "class A { self() { return A; } } var K = A; A = 1; (K.prototype.self() === K) + ':' + A;"
        ),
        Ok(Value::String("true:1".to_owned().into()))
    );
    assert_eq!(
        eval("class A { name2() { return A.name; } } A.prototype.name2();"),
        Ok(Value::String("A".to_owned().into()))
    );
    // Through a nested closure.
    assert_eq!(
        eval(
            "class A { boxed() { var f = function () { return A; }; return f(); } } var K = A; A = 0; K.prototype.boxed() === K;"
        ),
        Ok(Value::Boolean(true))
    );
    // Through a direct eval.
    assert_eq!(
        eval("class A { ev() { return eval('A'); } } var K = A; A = 0; K.prototype.ev() === K;"),
        Ok(Value::Boolean(true))
    );
    // Static methods, accessors, constructors, and field initializers.
    assert_eq!(
        eval(
            "class A { static make() { return new A(); } } var K = A; A = 0; K.make() instanceof K;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("class A { get own() { return A; } } var K = A; A = 0; (new K()).own === K;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("class A { constructor() { this.k = A; } } var K = A; A = 0; (new K()).k === K;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("class A { k = A; } var K = A; A = 0; (new K()).k === K;"),
        Ok(Value::Boolean(true))
    );
    // A named class expression, and a member that shadows the inner name.
    assert_eq!(
        eval("var C = class Inner { self() { return Inner; } }; C.prototype.self() === C;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("class A { shadow(A) { return A; } } A.prototype.shadow(7);"),
        Ok(Value::Number(7.0))
    );
    // The common case: a member that never mentions the name still runs.
    assert_eq!(
        eval(
            "class A { constructor(v) { this.v = v; } get2() { return this.v * 2; } } (new A(21)).get2();"
        ),
        Ok(Value::Number(42.0))
    );
}
