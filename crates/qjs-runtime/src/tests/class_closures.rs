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
