use crate::{Value, eval};

fn string(source: &str) -> String {
    match eval(source) {
        Ok(Value::String(value)) => value.to_string(),
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
fn iterator_constructor_uses_new_target_realm_default_prototype() {
    assert_eq!(
        eval(
            "let realmPrototype = {}; \
             function C() {} \
             Object.defineProperty(C, '__quickjsRustRealmIteratorPrototype', { value: realmPrototype }); \
             C.prototype = null; \
             Object.getPrototypeOf(Reflect.construct(Iterator, [], C)) === realmPrototype;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn iterator_zip_basic_modes_and_argument_validation() {
    assert_eq!(
        string(
            "let shortest = Iterator.zip([[1, 2], ['a']]).next().value.join(','); \
             let longest = Iterator.zip([[1], ['a', 'b']], { mode: 'longest', padding: ['x', 'y'] }); \
             let first = longest.next().value.join(','); \
             let second = longest.next().value.join(','); \
             let strict = Iterator.zip([[1], ['a']], { mode: 'strict' }).next().value.join(','); \
             let errors = []; \
             for (let value of [undefined, '', Symbol()]) { \
               try { Iterator.zip(value); } catch (e) { errors.push(e instanceof TypeError); } \
             } \
             try { Iterator.zip([], Symbol()); } catch (e) { errors.push(e instanceof TypeError); } \
             shortest + '|' + first + '|' + second + '|' + strict + '|' + errors.join(',');"
        ),
        "1,a|1,a|x,b|1,a|true,true,true,true"
    );
}

#[test]
fn iterator_zip_closes_open_iterators_in_reverse_order() {
    assert_eq!(
        string(
            "let log = []; \
             let first = { next() { log.push('first next'); return { done: false }; }, return() { log.push('first return'); return {}; } }; \
             let second = { next() { log.push('second next'); return { done: true }; }, return() { log.push('second return'); return {}; } }; \
             let third = { next() { log.push('third next'); return { done: false }; }, return() { log.push('third return'); throw new Error('ignored'); } }; \
             let it = Iterator.zip([first, second, third], { mode: 'strict' }); \
             try { it.next(); } catch (e) { log.push(e instanceof TypeError); } \
             log.join('|');"
        ),
        "first next|second next|third return|first return|true"
    );
}

#[test]
fn iterator_zip_array_fast_path_preserves_array_iteration_observability() {
    assert_eq!(
        string(
            "let input = [1]; \
             let iter = Iterator.zip([input]); \
             input.push(2); \
             let first = iter.next().value[0]; \
             let second = iter.next().value[0]; \
             let done = iter.next().done; \
             let inherited; \
             Array.prototype[0] = 'proto'; \
             try { inherited = Iterator.zip([[, 3]]).next().value[0]; } \
             finally { delete Array.prototype[0]; } \
             [first, second, done, inherited].join('|');"
        ),
        "1|2|true|proto"
    );
}

#[test]
fn iterator_zip_array_fast_path_observes_length_shrink() {
    assert_eq!(
        string(
            "let input = [1, 2]; \
             let iter = Iterator.zip([input]); \
             let first = iter.next().value[0]; \
             input.length = 1; \
             [first, iter.next().done].join('|');"
        ),
        "1|true"
    );
}

#[test]
fn iterator_zip_array_fast_path_respects_custom_array_iterator() {
    assert_eq!(
        string(
            "let inner = [1]; \
             let result; \
             inner[Symbol.iterator] = function() { \
               let done = false; \
               return { next() { if (done) return { done: true }; done = true; return { value: 'custom', done: false }; } }; \
             }; \
             result = Iterator.zip([inner]).next().value[0]; \
             result;"
        ),
        "custom"
    );
}

#[test]
fn iterator_zip_return_state_matches_suspended_state() {
    assert_eq!(
        string(
            "let log = []; \
             let startUnderlying = { next() { log.push('unexpected next'); }, return() { log.push('start return'); let r = start.return(); log.push(r.done); return {}; } }; \
             let start = Iterator.zip([startUnderlying]); \
             start.return(); \
             let yieldUnderlying = { next() { return { value: 1, done: false }; }, return() { log.push('yield return'); try { yielded.next(); } catch (e) { log.push(e instanceof TypeError); } return {}; } }; \
             let yielded = Iterator.zip([yieldUnderlying]); \
             yielded.next(); \
             yielded.return(); \
             log.join('|');"
        ),
        "start return|true|yield return|true"
    );
}

#[test]
fn iterator_zip_keyed_returns_null_prototype_property_objects() {
    assert_eq!(
        string(
            "let s = Symbol('s'); \
             let it = Iterator.zipKeyed({ [s]: ['S'], b: ['B'] }); \
             let value = it.next().value; \
             let keys = Reflect.ownKeys(value); \
             [Object.getPrototypeOf(value) === null, keys[0], keys[1] === s, value.b, value[s], it.next().done].join('|');"
        ),
        "true|b|true|B|S|true"
    );
}

#[test]
fn iterator_zip_keyed_skips_undefined_and_observes_enumerability_at_key_time() {
    assert_eq!(
        string(
            "let iterables = {}; \
             Object.defineProperty(iterables, 'a', { enumerable: true, get() { delete iterables.b; return ['A']; } }); \
             Object.defineProperty(iterables, 'b', { enumerable: true, value: ['B'], configurable: true }); \
             Object.defineProperty(iterables, 'd', { enumerable: true, get() { Object.defineProperty(iterables, 'c', { enumerable: true }); return undefined; } }); \
             Object.defineProperty(iterables, 'c', { enumerable: false, value: ['C'], configurable: true }); \
             let value = Iterator.zipKeyed(iterables).next().value; \
             Reflect.ownKeys(value).join(',') + '|' + value.a + '|' + ('b' in value) + '|' + ('c' in value) + '|' + ('d' in value);"
        ),
        "a,c|A|false|true|false"
    );
}

#[test]
fn iterator_zip_keyed_longest_reads_padding_by_key_and_closes_on_abrupt_get() {
    assert_eq!(
        string(
            "let log = []; \
             let first = { next() { log.push('unexpected first next'); }, return() { log.push('first return'); return {}; } }; \
             let second = { next() { log.push('unexpected second next'); }, return() { log.push('second return'); return {}; } }; \
             let padding = { get first() { log.push('padding first'); }, get second() { log.push('padding second'); throw new Error('padding'); } }; \
             try { Iterator.zipKeyed({ first, second }, { mode: 'longest', padding }); } catch (e) { log.push(e.message); } \
             log.join('|');"
        ),
        "padding first|padding second|second return|first return|padding"
    );
}

#[test]
fn iterator_zip_keyed_modes_match_zip_iteration() {
    assert_eq!(
        string(
            "let shortest = Iterator.zipKeyed({ a: [1, 2], b: ['x'] }); \
             let longest = Iterator.zipKeyed({ a: [1], b: ['x', 'y'] }, { mode: 'longest', padding: { a: 'pad-a' } }); \
             let strict = Iterator.zipKeyed({ a: [1], b: ['x'] }, { mode: 'strict' }); \
             let s0 = shortest.next().value; \
             let l0 = longest.next().value; \
             let l1 = longest.next().value; \
             let q0 = strict.next().value; \
             [s0.a, s0.b, shortest.next().done, l0.a, l0.b, l1.a, l1.b, q0.a, q0.b].join('|');"
        ),
        "1|x|true|1|x|pad-a|y|1|x"
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
fn iterator_from_accepts_direct_iterators_and_symbol_fallback() {
    assert_eq!(
        string(
            "function* g() { yield 0; yield 1; yield 2; } \
             let results = []; \
             let n = g(); \
             results.push(Array.from(Iterator.from({ next() { return n.next(); } })).join(',')); \
             n = g(); \
             results.push(Array.from(Iterator.from({ [Symbol.iterator]: null, next() { return n.next(); } })).join(',')); \
             n = g(); \
             results.push(Array.from(Iterator.from({ [Symbol.iterator]: undefined, next() { return n.next(); } })).join(',')); \
             let threw = false; \
             try { Iterator.from({ [Symbol.iterator]: 0, next() { return { done: true }; } }); } catch (e) { threw = e instanceof TypeError; } \
             results.push(threw); \
             threw = false; \
             try { Iterator.from(Symbol()); } catch (e) { threw = e instanceof TypeError; } \
             results.push(threw); \
             results.join(':');"
        ),
        "0,1,2:0,1,2:0,1,2:true:true"
    );
}

#[test]
fn iterator_from_gets_direct_next_once() {
    assert_eq!(
        string(
            "let nextGets = 0; \
             let nextCalls = 0; \
             let source = { \
               get next() { \
                 nextGets++; \
                 let value = 0; \
                 return function () { nextCalls++; return value++ < 2 ? { value, done: false } : { done: true }; }; \
               } \
             }; \
             let iter = Iterator.from(source); \
             let afterFrom = nextGets + ':' + nextCalls; \
             Array.from(iter); \
             afterFrom + ':' + nextGets + ':' + nextCalls;"
        ),
        "1:0:1:3"
    );
}

#[test]
fn iterator_from_wrapper_return_forwards_or_creates_done_result() {
    assert_eq!(
        string(
            "let log = []; \
             let expected = { value: 5, done: true }; \
             let source = { \
               get return() { log.push('get return'); return function () { log.push('call return'); return expected; }; } \
             }; \
             let wrapper = Iterator.from(source); \
             let result = wrapper.return(); \
             let emptyReturn = Iterator.from({}).return(); \
             (result === expected) + ':' + log.join('|') + ':' + \
             emptyReturn.hasOwnProperty('value') + ':' + (emptyReturn.value === undefined) + ':' + emptyReturn.done;"
        ),
        "true:get return|call return:true:true:true"
    );
}

#[test]
fn iterator_from_observes_proxy_iterator_methods_in_order() {
    assert_eq!(
        string(
            "let log = []; \
             let expected = { value: 5, done: true }; \
             let source = new Proxy({ return() { log.push('call return'); return expected; } }, { \
               get(target, key, receiver) { \
                 log.push(key === Symbol.iterator ? 'get @@iterator' : 'get ' + String(key)); \
                 return Reflect.get(target, key, receiver); \
               } \
             }); \
             let wrapper = Iterator.from(source); \
             let before = log.join('|'); \
             let result = wrapper.return(); \
             (result === expected) + ':' + before + ':' + log.join('|');"
        ),
        "true:get @@iterator|get next:get @@iterator|get next|get return|call return"
    );
}

#[test]
fn iterator_concat_static_surface_and_basic_iteration() {
    assert_eq!(
        string(
            "let desc = Object.getOwnPropertyDescriptor(Iterator, 'concat'); \
             let length = Object.getOwnPropertyDescriptor(Iterator.concat, 'length'); \
             let name = Object.getOwnPropertyDescriptor(Iterator.concat, 'name'); \
             let iter = Iterator.concat([1, 2], new Set([3])); \
             [typeof Iterator.concat, Iterator.concat.length, Iterator.concat.name, \
              desc.writable, desc.enumerable, desc.configurable, \
              length.writable, length.enumerable, length.configurable, \
              name.writable, name.enumerable, name.configurable, \
              iter instanceof Iterator, Array.from(iter).join(',')].join(':');"
        ),
        "function:0:concat:true:false:true:false:false:true:false:false:true:true:1,2,3"
    );
}

#[test]
fn iterator_concat_reads_methods_once_and_opens_iterators_lazily() {
    assert_eq!(
        string(
            "let log = []; \
             let first = { \
               get [Symbol.iterator]() { log.push('get first'); return function () { log.push('open first'); return [1][Symbol.iterator](); }; } \
             }; \
             let second = { \
               get [Symbol.iterator]() { log.push('get second'); return function () { log.push('open second'); return [2][Symbol.iterator](); }; } \
             }; \
             let iter = Iterator.concat(first, second); \
             let afterCreate = log.join('|'); \
             let a = iter.next(); \
             let afterFirst = log.join('|'); \
             let b = iter.next(); \
             let c = iter.next(); \
             [afterCreate, afterFirst, log.join('|'), a.value, b.value, c.done].join(':');"
        ),
        "get first|get second:get first|get second|open first:get first|get second|open first|open second:1:2:true"
    );
}

#[test]
fn iterator_concat_return_closes_only_started_inner_iterator() {
    assert_eq!(
        string(
            "let returns = 0; \
             let opened = 0; \
             let active = { \
               next() { return { done: false, value: 1 }; }, \
               return() { returns++; return {}; } \
             }; \
             let iter = Iterator.concat({ [Symbol.iterator]() { opened++; return active; } }); \
             iter.return(); \
             let beforeStart = opened + ':' + returns; \
             iter = Iterator.concat({ [Symbol.iterator]() { opened++; return active; } }); \
             iter.next(); \
             iter.return(); \
             iter.return(); \
             beforeStart + ':' + opened + ':' + returns;"
        ),
        "0:0:1:1"
    );
}

#[test]
fn iterator_concat_return_rejects_reentry() {
    assert_eq!(
        string(
            "let enterCount = 0; \
             let source = { \
               next() { return { done: false }; }, \
               return() { enterCount++; iter.return(); return {}; } \
             }; \
             let iter = Iterator.concat({ [Symbol.iterator]() { return source; } }); \
             iter.next(); \
             let threw = false; \
             try { iter.return(); } catch (e) { threw = e instanceof TypeError; } \
             threw + ':' + enterCount;"
        ),
        "true:1"
    );
}

#[test]
fn iterator_concat_does_not_read_done_value() {
    assert_eq!(
        string(
            "let valueGets = 0; \
             let source = { \
               [Symbol.iterator]() { \
                 return { next() { return { get value() { valueGets++; }, done: true }; } }; \
               } \
             }; \
             let result = Iterator.concat(source, source).next(); \
             result.done + ':' + (result.value === undefined) + ':' + valueGets;"
        ),
        "true:true:0"
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
fn iterator_lazy_helpers_cache_underlying_next_method() {
    assert_eq!(
        string(
            "let methods = ['map', 'filter', 'take', 'drop', 'flatMap']; \
             let results = []; \
             for (let method of methods) { \
               let nextGets = 0; \
               let nextCalls = 0; \
               class CountingIterator extends Iterator { \
                 get next() { \
                   nextGets++; \
                   let iter = (function* () { \
                     for (let i = 1; i < 5; ++i) { yield i; } \
                   })(); \
                   return function() { \
                     nextCalls++; \
                     return iter.next(); \
                   }; \
                 } \
               } \
               let iterator = new CountingIterator(); \
               let helper = method === 'map' ? iterator.map(x => x) : \
                 method === 'filter' ? iterator.filter(() => true) : \
                 method === 'take' ? iterator.take(2) : \
                 method === 'drop' ? iterator.drop(2) : \
                 iterator.flatMap(x => [x]); \
               for (const value of helper) { } \
               results.push(method + ':' + nextGets + ':' + nextCalls); \
             } \
             results.join('|');"
        ),
        "map:1:5|filter:1:5|take:1:2|drop:1:5|flatMap:1:5"
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

#[test]
fn iterator_prototype_symbol_dispose_surface() {
    assert_eq!(
        string(
            "let IteratorPrototype = Object.getPrototypeOf(Object.getPrototypeOf([][Symbol.iterator]())); \
             let method = IteratorPrototype[Symbol.dispose]; \
             let desc = Object.getOwnPropertyDescriptor(IteratorPrototype, Symbol.dispose); \
             let length = Object.getOwnPropertyDescriptor(method, 'length'); \
             let name = Object.getOwnPropertyDescriptor(method, 'name'); \
             [typeof method, method.length, method.name, desc.writable, desc.enumerable, desc.configurable, \
              length.writable, length.enumerable, length.configurable, name.writable, name.enumerable, name.configurable].join(':');"
        ),
        "function:0:[Symbol.dispose]:true:false:true:false:false:true:false:false:true"
    );
}

#[test]
fn iterator_prototype_symbol_dispose_invokes_return_and_returns_undefined() {
    assert_eq!(
        string(
            "let IteratorPrototype = Object.getPrototypeOf(Object.getPrototypeOf([][Symbol.iterator]())); \
             let iter = Object.create(IteratorPrototype); \
             let returnCalled = false; \
             iter.return = function () { returnCalled = true; return { done: true }; }; \
             let result = iter[Symbol.dispose](); \
             returnCalled + ':' + (result === undefined) + ':' + (IteratorPrototype[Symbol.dispose]() === undefined);"
        ),
        "true:true:true"
    );
}
