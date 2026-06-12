use crate::{Value, eval, promise};

#[test]
fn evaluates_array_constructor_length_argument() {
    assert_eq!(eval("new Array(3).length;"), Ok(Value::Number(3.0)));
    assert_eq!(eval("new Array(3)[0];"), Ok(Value::Undefined));
    assert_eq!(
        eval("let value = new Array('3'); value.length + ':' + value[0];"),
        Ok(Value::String("1:3".to_owned()))
    );
    assert_eq!(
        eval(
            "let caught = false; try { new Array(1.5); } catch (error) { caught = error instanceof RangeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let caught = false; try { new Array(Number.MAX_VALUE); } catch (error) { caught = error instanceof RangeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn evaluates_array_of_static_constructor() {
    assert_eq!(eval("Array.of.length;"), Ok(Value::Number(0.0)));
    assert_eq!(
        eval(
            "let values = Array.of(1, 'x', true, null, undefined); values.length + ':' + values[0] + ':' + values[1] + ':' + values[2] + ':' + (values[3] === null) + ':' + (values[4] === undefined);"
        ),
        Ok(Value::String("5:1:x:true:true:true".to_owned()))
    );
    assert_eq!(eval("Array.of(3).length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Array.of(3)[0];"), Ok(Value::Number(3.0)));
    assert_eq!(
        eval("Array.isArray(Array.of(1, 2));"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "function Coop() {} let coop = Array.of.call(Coop, 'a', 'b'); (coop instanceof Coop) + ':' + coop.length + ':' + coop[0] + ':' + coop[1];"
        ),
        Ok(Value::String("true:2:a:b".to_owned()))
    );
    assert_eq!(
        eval(
            "function T() { Object.preventExtensions(this); } let caught = false; try { Array.of.call(T, 'x'); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "function T() { Object.defineProperty(this, 0, { configurable: false, writable: true, enumerable: true }); } let caught = false; try { Array.of.call(T, 'x'); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "function C() {} C.prototype = null; Object.getPrototypeOf(Array.of.call(C, 1, 2, 3)) === Object.prototype;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let marker = {}; function T() { return new Proxy({}, { defineProperty: function() { throw marker; } }); } let caught = false; try { Array.of.call(T, 'Bob'); } catch (error) { caught = error === marker; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let hits = 0; let value = 0; function Pack() { Object.defineProperty(this, 'length', { set: function(next) { hits = hits + 1; value = next; } }); } Array.of.call(Pack, 'a', 'b'); hits + ':' + value;"
        ),
        Ok(Value::String("1:2".to_owned()))
    );
    assert_eq!(
        eval(
            "function Pack() { Object.defineProperty(this, 'length', { set: function() { throw 'length'; } }); } let caught = false; try { Array.of.call(Pack); } catch (error) { caught = error === 'length'; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn evaluates_array_from_static_constructor() {
    assert_eq!(eval("Array.from.length;"), Ok(Value::Number(1.0)));
    assert_eq!(
        eval(
            "let source = [0, 'foo', undefined, Infinity]; let result = Array.from(source); result.length + ':' + result[0] + ':' + result[1] + ':' + (result[2] === undefined) + ':' + result[3] + ':' + (result === source);"
        ),
        Ok(Value::String("4:0:foo:true:Infinity:false".to_owned()))
    );
    assert_eq!(
        eval("Array.from('Test').join('');"),
        Ok(Value::String("Test".to_owned()))
    );
    assert_eq!(
        eval("Array.from({ length: 3, 0: 'a', 2: 'c' }).join('|');"),
        Ok(Value::String("a||c".to_owned()))
    );
    assert_eq!(
        eval("Array.from.call(Object, []).constructor === Object;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "function C(length) { this.args = arguments; } let result = Array.from.call(C, { length: 42 }); result instanceof C && result.args.length === 1 && result.args[0] === 42 && result.length === 42;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "function C() { Object.defineProperty(this, '0', { value: 1, writable: false, configurable: true }); } let result = Array.from.call(C, { length: 1, 0: 2 }); let desc = Object.getOwnPropertyDescriptor(result, '0'); result[0] + ':' + desc.writable + ':' + desc.enumerable + ':' + desc.configurable;"
        ),
        Ok(Value::String("2:true:true:true".to_owned()))
    );
    assert_eq!(
        eval(
            "function C() { Object.preventExtensions(this); } let caught = false; try { Array.from.call(C, { length: 1 }); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "function C() {} C.prototype = null; Object.getPrototypeOf(Array.from.call(C, [])) === Object.prototype;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn exposes_array_from_async_static_surface() {
    assert_eq!(
        eval("typeof Array.fromAsync + ':' + Array.fromAsync.length + ':' + Array.fromAsync.name;"),
        Ok(Value::String("function:1:fromAsync".to_owned()))
    );
    assert_eq!(
        eval(
            "let desc = Object.getOwnPropertyDescriptor(Array, 'fromAsync'); desc.writable + ':' + desc.enumerable + ':' + desc.configurable;"
        ),
        Ok(Value::String("true:false:true".to_owned()))
    );
    assert_eq!(
        eval("Array.fromAsync.hasOwnProperty('prototype');"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval(
            "let caught = false; try { Reflect.construct(function() {}, [], Array.fromAsync); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Array.fromAsync([]) instanceof Promise;"),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn array_from_async_resolves_array_like_and_sync_iterable_inputs() {
    assert_eq!(
        promise::promise_debug_state_result(
            &eval(
                "Array.fromAsync({ length: 3, 0: 'a', 1: 'b' }).then(function(value) { return value.length + ':' + value[0] + ':' + value[1] + ':' + value[2]; });"
            )
            .unwrap()
        ),
        Some((
            "fulfilled".to_owned(),
            Value::String("3:a:b:undefined".to_owned())
        ))
    );
    assert_eq!(
        promise::promise_debug_state_result(
            &eval("Array.fromAsync('hi', function(value, index) { return value + index; }).then(function(value) { return value.join(''); });")
                .unwrap()
        ),
        Some(("fulfilled".to_owned(), Value::String("h0i1".to_owned())))
    );
    assert_eq!(
        promise::promise_debug_state_result(
            &eval(
                "Number.prototype.length = 2; Number.prototype[0] = 'x'; Number.prototype[1] = 'y'; Array.fromAsync(1).then(function(value) { return value.join(''); });"
            )
            .unwrap()
        ),
        Some(("fulfilled".to_owned(), Value::String("xy".to_owned())))
    );
    assert_eq!(
        promise::promise_debug_state_result(
            &eval(
                "BigInt.prototype.length = 2; BigInt.prototype[0] = 1; BigInt.prototype[1] = 2; Array.fromAsync(1n).then(function(value) { return value.join(':'); });"
            )
            .unwrap()
        ),
        Some(("fulfilled".to_owned(), Value::String("1:2".to_owned())))
    );
}

#[test]
fn array_from_async_awaits_thenable_inputs_and_map_results() {
    assert_eq!(
        promise::promise_debug_state_result(
            &eval(
                "let count = 0; \
                 let thenable = { then(resolve) { count++; resolve(7); } }; \
                 Array.fromAsync({ length: 1, 0: thenable }).then(function() { return count; });"
            )
            .unwrap()
        ),
        Some(("fulfilled".to_owned(), Value::Number(1.0)))
    );
    assert_eq!(
        promise::promise_debug_state_result(
            &eval(
                "let count = 0; \
                 let thenable = { then(resolve) { count++; resolve(8); } }; \
                 Array.fromAsync([1], function() { return thenable; }).then(function(value) { return count + ':' + value[0]; });"
            )
            .unwrap()
        ),
        Some(("fulfilled".to_owned(), Value::String("1:8".to_owned())))
    );
}

#[test]
fn array_from_async_rejects_early_errors() {
    assert_eq!(
        promise::promise_debug_state_result(
            &eval(
                "Array.fromAsync([], null).then(null, function(error) { return error instanceof TypeError; });"
            )
            .unwrap()
        ),
        Some(("fulfilled".to_owned(), Value::Boolean(true)))
    );
    assert_eq!(
        promise::promise_debug_state_result(
            &eval(
                "Array.fromAsync(null).then(null, function(error) { return error instanceof TypeError; });"
            )
            .unwrap()
        ),
        Some(("fulfilled".to_owned(), Value::Boolean(true)))
    );
    assert_eq!(
        promise::promise_debug_state_result(
            &eval(
                "Array.fromAsync({ get length() { throw new RangeError('boom'); } }).then(null, function(error) { return error instanceof RangeError; });"
            )
            .unwrap()
        ),
        Some(("fulfilled".to_owned(), Value::Boolean(true)))
    );
    assert_eq!(
        promise::promise_debug_state_result(
            &eval(
                "Array.fromAsync.call({}, { length: 4294967296 }).then(null, function(error) { return error instanceof RangeError; });"
            )
            .unwrap()
        ),
        Some(("fulfilled".to_owned(), Value::Boolean(true)))
    );
}

#[test]
fn exposes_array_species_accessor() {
    assert_eq!(
        eval(
            "let desc = Object.getOwnPropertyDescriptor(Array, Symbol.species); let receiver = {}; [desc.get.call(receiver) === receiver, desc.set === undefined, desc.enumerable, desc.configurable, desc.get.name, desc.get.length].join(':');"
        ),
        Ok(Value::String(
            "true:true:false:true:get [Symbol.species]:0".to_owned()
        ))
    );
}

#[test]
fn evaluates_array_from_mapping() {
    assert_eq!(
        eval("Array.from([1, 2], function(value, index) { return value + index; }).join();"),
        Ok(Value::String("1,3".to_owned()))
    );
    assert_eq!(
        eval("Array.from([1], function(value) { return value + this.offset; }, { offset: 4 })[0];"),
        Ok(Value::Number(5.0))
    );
    assert!(eval("Array.from([1], null);").is_err());
    assert!(eval("Array.from(null);").is_err());
}

#[test]
fn evaluates_array_from_iterables() {
    assert_eq!(
        eval("Array.from(new Set(['a', 'b'])).join('|');"),
        Ok(Value::String("a|b".to_owned()))
    );
    assert_eq!(
        eval(
            "let source = { length: 1, 0: 'array-like' }; source[Symbol.iterator] = function() { return ['iterable'][Symbol.iterator](); }; Array.from(source)[0];"
        ),
        Ok(Value::String("iterable".to_owned()))
    );
    assert_eq!(
        eval(
            "let source = {}; source[Symbol.iterator] = function() { let index = 0; return { next: function() { index = index + 1; return index > 2 ? { done: true } : { value: index * 3, done: false }; } }; }; Array.from(source).join();"
        ),
        Ok(Value::String("3,6".to_owned()))
    );
    assert_eq!(
        eval(
            "let log = ''; function C() { log += 'c'; } let source = {}; source[Symbol.iterator] = function() { log += 'i'; return { next: function() { return { done: true }; } }; }; let result = Array.from.call(C, source); log + ':' + (result instanceof C);"
        ),
        Ok(Value::String("ci:true".to_owned()))
    );
    assert_eq!(
        eval(
            "let marker = {}; function C() { throw marker; } let source = {}; source[Symbol.iterator] = function() { return { next: function() { throw {}; } }; }; let caught = false; try { Array.from.call(C, source); } catch (error) { caught = error === marker; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn maps_array_from_iterables_during_consumption() {
    assert_eq!(
        eval(
            "let log = ''; let source = {}; source[Symbol.iterator] = function() { let index = 0; return { next: function() { log = log + 'n' + index; index = index + 1; return index > 2 ? { done: true } : { value: index, done: false }; } }; }; Array.from(source, function(value, index) { log = log + 'm' + index; return value; }); log;"
        ),
        Ok(Value::String("n0m0n1m1n2".to_owned()))
    );
    assert_eq!(
        eval(
            "Array.from(new Set([1, 2]), function(value, index) { return value + index + this.offset; }, { offset: 4 }).join();"
        ),
        Ok(Value::String("5,7".to_owned()))
    );
    assert_eq!(
        eval(
            "let closeCount = 0; let marker = {}; let source = {}; source[Symbol.iterator] = function() { return { return: function() { closeCount += 1; return {}; }, next: function() { return { value: 1, done: false }; } }; }; let caught = false; try { Array.from(source, function() { throw marker; }); } catch (error) { caught = error === marker; } caught + ':' + closeCount;"
        ),
        Ok(Value::String("true:1".to_owned()))
    );
    assert_eq!(
        eval(
            "let marker = {}; function C() {} Object.defineProperty(C.prototype, 'length', { set: function(_) { throw marker; } }); let source = {}; source[Symbol.iterator] = function() { return { next: function() { return { done: true }; } }; }; let caught = false; try { Array.from.call(C, source); } catch (error) { caught = error === marker; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert!(eval("let source = {}; source[Symbol.iterator] = 1; Array.from(source);").is_err());
}
