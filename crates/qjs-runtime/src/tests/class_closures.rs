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
