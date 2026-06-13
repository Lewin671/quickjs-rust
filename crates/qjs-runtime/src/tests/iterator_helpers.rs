use crate::{Value, eval};

fn string(source: &str) -> String {
    match eval(source) {
        Ok(Value::String(value)) => value,
        other => panic!("expected string, got {other:?}"),
    }
}

#[test]
fn iterator_take_argument_effect_order() {
    assert_eq!(
        string(
            "let effects = []; \
             Iterator.prototype.take.call( \
               { get next() { effects.push('get next'); return function () { return { done: true, value: undefined }; }; } }, \
               { valueOf() { effects.push('ToNumber limit'); return 0; } } \
             ); \
             effects.join('|');"
        ),
        "ToNumber limit|get next"
    );
    assert_eq!(
        string(
            "let effects = []; \
             let threw = false; \
             try { \
               Iterator.prototype.take.call(null, { valueOf() { effects.push('ToNumber limit'); return 0; } }); \
             } catch (e) { threw = e instanceof TypeError; } \
             threw + ':' + effects.length;"
        ),
        "true:0"
    );
    assert_eq!(
        string(
            "let effects = []; \
             let threw = false; \
             try { \
               Iterator.prototype.take.call( \
                 { get next() { effects.push('get next'); return function () { return { done: true, value: undefined }; }; } }, \
                 NaN \
               ); \
             } catch (e) { threw = e instanceof RangeError; } \
             threw + ':' + effects.length;"
        ),
        "true:0"
    );
}

#[test]
fn iterator_take_argument_validation_failure_closes_underlying() {
    assert_eq!(
        string(
            "let closed = false; \
             let closable = { \
               __proto__: Iterator.prototype, \
               get next() { throw new Error('next should not be read'); }, \
               return() { closed = true; return {}; } \
             }; \
             let r = []; \
             try { closable.take(); } catch (e) { r.push(e instanceof RangeError, closed); } \
             closed = false; \
             try { closable.take(NaN); } catch (e) { r.push(e instanceof RangeError, closed); } \
             closed = false; \
             try { closable.take(-1); } catch (e) { r.push(e instanceof RangeError, closed); } \
             closed = false; \
             try { closable.take({ get valueOf() { throw new Error('limit'); } }); } \
             catch (e) { r.push(e.message, closed); } \
             r.join(':');"
        ),
        "true:true:true:true:true:true:limit:true"
    );
}

#[test]
fn iterator_helper_next_rejects_reentry() {
    assert_eq!(
        string(
            "let enterCount = 0; \
             class TestIterator extends Iterator { \
               next() { enterCount++; iter.next(); return { done: false }; } \
             } \
             let iter = new TestIterator().take(100); \
             let threw = false; \
             try { iter.next(); } catch (e) { threw = e instanceof TypeError; } \
             threw + ':' + enterCount;"
        ),
        "true:1"
    );
}
