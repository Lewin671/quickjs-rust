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

#[test]
fn iterator_flat_map_argument_effect_order() {
    assert_eq!(
        string(
            "let effects = []; \
             let threw = false; \
             try { \
               Iterator.prototype.flatMap.call( \
                 { get next() { effects.push('get next'); return function () { return { done: true, value: undefined }; }; } }, \
                 { valueOf() { effects.push('valueOf mapper'); return function () { return []; }; } } \
               ); \
             } catch (e) { threw = e instanceof TypeError; } \
             threw + ':' + effects.length;"
        ),
        "true:0"
    );
}

#[test]
fn iterator_flat_map_argument_validation_failure_closes_underlying() {
    assert_eq!(
        string(
            "let closed = false; \
             let closable = { \
               __proto__: Iterator.prototype, \
               get next() { throw new Error('next should not be read'); }, \
               return() { closed = true; return {}; } \
             }; \
             let r = []; \
             try { closable.flatMap(); } catch (e) { r.push(e instanceof TypeError, closed); } \
             closed = false; \
             try { closable.flatMap({}); } catch (e) { r.push(e instanceof TypeError, closed); } \
             r.join(':');"
        ),
        "true:true:true:true"
    );
}

#[test]
fn iterator_flat_map_flattens_iterator_objects() {
    assert_eq!(
        string(
            "function* g() { yield 0; yield 1; yield 2; yield 3; } \
             let iter = g().flatMap((v) => { \
               let i = 0; \
               return { \
                 next() { \
                   if (i < v) { ++i; return { value: v, done: false }; } \
                   return { value: undefined, done: true }; \
                 } \
               }; \
             }); \
             Array.from(iter).join(',');"
        ),
        "1,2,2,3,3,3"
    );
}

#[test]
fn iterator_flat_map_iterator_symbol_fallback() {
    assert_eq!(
        string(
            "function* g() { yield 0; } \
             function* h() { yield 0; yield 1; yield 2; } \
             let r = []; \
             let iter = g().flatMap(() => { let n = h(); return { [Symbol.iterator]: 0, next: () => n.next() }; }); \
             try { iter.next(); } catch (e) { r.push(e instanceof TypeError); } \
             iter = g().flatMap(() => { let n = h(); return { [Symbol.iterator]: null, next: () => n.next() }; }); \
             r.push(Array.from(iter).join(',')); \
             iter = g().flatMap(() => { let n = h(); return { [Symbol.iterator]: undefined, next: () => n.next() }; }); \
             r.push(Array.from(iter).join(',')); \
             r.join(':');"
        ),
        "true:0,1,2:0,1,2"
    );
}

#[test]
fn iterator_flat_map_return_closes_inner_iterator_once() {
    assert_eq!(
        string(
            "let returnCount = 0; \
             function* g() { yield 0; } \
             let iter = g().flatMap(() => ({ \
               next() { return { done: false, value: 1 }; }, \
               return() { ++returnCount; return {}; } \
             })); \
             let first = iter.next(); \
             iter.return(); \
             iter.return(); \
             first.done + ':' + first.value + ':' + returnCount;"
        ),
        "false:1:1"
    );
}

#[test]
fn iterator_eager_helpers_validate_callback_before_next() {
    assert_eq!(
        string(
            "let methods = ['some', 'reduce', 'forEach', 'find', 'every']; \
             let results = []; \
             for (let method of methods) { \
               let effects = []; \
               let threw = false; \
               try { \
                 Iterator.prototype[method].call( \
                   { get next() { effects.push('get next'); return function () { return { done: true, value: undefined }; }; } }, \
                   null \
                 ); \
               } catch (e) { threw = e instanceof TypeError; } \
               results.push(method + ':' + threw + ':' + effects.length); \
             } \
             results.join('|');"
        ),
        "some:true:0|reduce:true:0|forEach:true:0|find:true:0|every:true:0"
    );
}

#[test]
fn iterator_eager_helpers_close_on_callback_validation_failure() {
    assert_eq!(
        string(
            "let methods = ['some', 'reduce', 'forEach', 'find', 'every']; \
             let results = []; \
             for (let method of methods) { \
               let closed = false; \
               let closable = { \
                 __proto__: Iterator.prototype, \
                 get next() { throw new Error('next should not be read'); }, \
                 return() { closed = true; return {}; } \
               }; \
               let threw = false; \
               try { closable[method]({}); } catch (e) { threw = e instanceof TypeError; } \
               results.push(method + ':' + threw + ':' + closed); \
             } \
             results.join('|');"
        ),
        "some:true:true|reduce:true:true|forEach:true:true|find:true:true|every:true:true"
    );
}
