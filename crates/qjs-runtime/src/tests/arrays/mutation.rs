use crate::{Value, eval};

#[test]
fn evaluates_array_mutation_builtins() {
    assert_eq!(
        eval("let xs = [1]; xs.push(2, 3) + ':' + xs.length + ':' + xs.join();"),
        Ok(Value::String("3:3:1,2,3".to_owned().into()))
    );
    assert_eq!(
        eval("let xs = [1, 2]; xs.length = 1; xs.length + ':' + xs.hasOwnProperty('1');"),
        Ok(Value::String("1:false".to_owned().into()))
    );
    assert_eq!(
        eval("let xs = [1, 2]; xs.length = 2.0; xs.length;"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval(
            "let caught = false; try { [].length = 4294967296; } catch (error) { caught = error instanceof RangeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let array = [1, 2, 3]; let hints = []; let length = {}; length[Symbol.toPrimitive] = function(hint) { hints.push(hint); Object.defineProperty(array, 'length', { writable: false }); return 0; }; let caught = false; try { (function() { 'use strict'; array.length = length; })(); } catch (error) { caught = error instanceof TypeError; } caught + ':' + hints.join() + ':' + array.length;"
        ),
        Ok(Value::String("true:number,number:3".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let caught = false; try { Reflect.set([], 'length', -1); } catch (error) { caught = error instanceof RangeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let object = { 0: 1, length: 1 }; Array.prototype.push.call(object, 2, 3) + ':' + object.length + ':' + object[2];"
        ),
        Ok(Value::String("3:3:3".to_owned().into()))
    );
    assert_eq!(
        eval("Array.prototype.push.call(false);"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval(
            "let caught = false; try { Array.prototype.push.call('', 1); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("let xs = [1, 2]; xs.pop() + ':' + xs.length + ':' + xs.join();"),
        Ok(Value::String("2:1:1".to_owned().into()))
    );
    assert_eq!(eval("[].pop();"), Ok(Value::Undefined));
    assert_eq!(
        eval(
            "let object = { 0: 1, 1: 2, length: 2 }; Array.prototype.pop.call(object) + ':' + object.length + ':' + object.hasOwnProperty('1');"
        ),
        Ok(Value::String("2:1:false".to_owned().into()))
    );
    assert_eq!(
        eval("Array.prototype.pop.call(false);"),
        Ok(Value::Undefined)
    );
    assert_eq!(
        eval(
            "let caughtEmpty = false; let caughtText = false; try { Array.prototype.pop.call(''); } catch (error) { caughtEmpty = error instanceof TypeError; } try { Array.prototype.pop.call('abc'); } catch (error) { caughtText = error instanceof TypeError; } caughtEmpty + ':' + caughtText;"
        ),
        Ok(Value::String("true:true".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let proto = { length: 2 }; let object = Object.create(proto); object[1] = 7; Array.prototype.pop.call(object) + ':' + object.length + ':' + proto.length;"
        ),
        Ok(Value::String("7:1:2".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let calls = 0; let object = { 0: 1, length: 1 }; Object.defineProperty(object, 'length', { set: function(value) { calls = value + 1; }, configurable: true }); Array.prototype.pop.call(object); calls;"
        ),
        Ok(Value::Number(1.0))
    );
    assert!(
        eval("let object = { length: 1 }; Object.defineProperty(object, '0', { value: 1 }); Array.prototype.pop.call(object);").is_err()
    );
    assert!(
        eval("let object = { 0: 1 }; Object.defineProperty(object, 'length', { value: 1 }); Array.prototype.pop.call(object);").is_err()
    );
    assert_eq!(
        eval("let xs = [1, 2]; xs.shift() + ':' + xs.length + ':' + xs.join();"),
        Ok(Value::String("1:1:2".to_owned().into()))
    );
    assert_eq!(eval("[].shift();"), Ok(Value::Undefined));
    assert_eq!(
        eval(
            "let object = { 0: 1, 1: 2, 2: 3, length: 3 }; Array.prototype.shift.call(object) + ':' + object.length + ':' + object[0] + ':' + object[1] + ':' + object.hasOwnProperty('2');"
        ),
        Ok(Value::String("1:2:2:3:false".to_owned().into()))
    );
    assert_eq!(
        eval("Array.prototype.shift.call(false);"),
        Ok(Value::Undefined)
    );
    assert_eq!(
        eval(
            "let caught = false; try { Array.prototype.shift.call('abc'); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let caught = false; let array = [1, 2]; Object.freeze(array); try { Array.prototype.shift.call(array); } catch (error) { caught = error instanceof TypeError; } caught + ':' + array.length + ':' + array.join();"
        ),
        Ok(Value::String("true:2:1,2".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let caught = false; let array = new Array(1); Object.defineProperty(Array.prototype, '0', { get: function() { Object.defineProperty(array, 'length', { writable: false }); }, configurable: true }); try { array.shift(); } catch (error) { caught = error instanceof TypeError; } delete Array.prototype[0]; caught + ':' + array.length;"
        ),
        Ok(Value::String("true:1".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let caught = false; let object = { 0: 1, length: 1 }; Object.defineProperty(object, 'length', { value: 1 }); try { Array.prototype.shift.call(object); } catch (error) { caught = error instanceof TypeError; } caught + ':' + object.length;"
        ),
        Ok(Value::String("false:0".to_owned().into()))
    );
    assert_eq!(
        eval("let xs = [3]; xs.unshift(1, 2) + ':' + xs.join();"),
        Ok(Value::String("3:1,2,3".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let object = { 0: 1, 1: 2, length: 2 }; Array.prototype.unshift.call(object, -1, 0) + ':' + object.length + ':' + object[0] + ':' + object[1] + ':' + object[2] + ':' + object[3];"
        ),
        Ok(Value::String("4:4:-1:0:1:2".to_owned().into()))
    );
    assert_eq!(
        eval("Array.prototype.unshift.call(false, 1);"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval(
            "let caught = false; try { Array.prototype.unshift.call('abc', 1); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let array = []; let calls = 0; Object.defineProperty(Array.prototype, '0', { set: function(value) { calls = value; Object.defineProperty(array, 'length', { writable: false }); }, configurable: true }); let caught = false; try { array.unshift(1); } catch (error) { caught = error instanceof TypeError; } delete Array.prototype[0]; caught + ':' + calls + ':' + array.hasOwnProperty('0') + ':' + array.length;"
        ),
        Ok(Value::String("true:1:false:0".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let caught = false; let array = []; Object.defineProperty(array, 'length', { writable: false }); try { array.unshift(); } catch (error) { caught = error instanceof TypeError; } caught + ':' + array.length;"
        ),
        Ok(Value::String("true:0".to_owned().into()))
    );
    assert!(
        eval("let object = {}; Object.defineProperty(object, '0', { get: function() {} }); Array.prototype.unshift.call(object, 0);").is_err()
    );
    assert_eq!(
        eval("let xs = [1, 2, 3]; let result = xs.reverse(); result === xs && xs.join();"),
        Ok(Value::String("3,2,1".to_owned().into()))
    );
    assert_eq!(
        eval("Array.prototype.reverse.call(true) instanceof Boolean;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let object = { length: 4, 0: 'a', 2: 'c' }; let result = Array.prototype.reverse.call(object); (result === object) + ':' + object[1] + ':' + object[3] + ':' + object.hasOwnProperty('0') + ':' + object.hasOwnProperty('2');"
        ),
        Ok(Value::String("true:c:a:false:false".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let traps = []; \
             let target = { 0: 'a', 1: 'b', length: 2 }; \
             let proxy = new Proxy(target, { \
               set: function(t, key, value, receiver) { traps.push('set:' + key + ':' + value); return Reflect.set(t, key, value, receiver); } \
             }); \
             Array.prototype.reverse.call(proxy); \
             traps.join('|') + ':' + target[0] + ':' + target[1];"
        ),
        Ok(Value::String("set:0:b|set:1:a:b:a".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let array = ['first', 'second']; Object.defineProperty(array, '0', { get: function() { array.length = 0; return 'first'; }, configurable: true }); array.reverse(); (0 in array) + ':' + (1 in array) + ':' + array[1];"
        ),
        Ok(Value::String("false:true:first".to_owned().into()))
    );
    assert_eq!(
        eval(
            "Array.prototype[1] = 1; let array = [0]; array.length = 2; array.reverse(); let out = array[0] + ':' + array[1] + ':' + array.hasOwnProperty('0') + ':' + array.hasOwnProperty('1'); delete Array.prototype[1]; out;"
        ),
        Ok(Value::String("1:0:true:true".to_owned().into()))
    );
    assert_eq!(
        eval("let xs = [1, 2, 3]; let result = xs.fill(9); result === xs && xs.join();"),
        Ok(Value::String("9,9,9".to_owned().into()))
    );
    assert_eq!(
        eval("[1, 2, 3, 4].fill(0, 1, 3).join();"),
        Ok(Value::String("1,0,0,4".to_owned().into()))
    );
    assert_eq!(
        eval("[1, 2, 3, 4].fill(0, -3, -1).join();"),
        Ok(Value::String("1,0,0,4".to_owned().into()))
    );
    assert_eq!(
        eval("[1, 2, 3].fill(0, 5).join();"),
        Ok(Value::String("1,2,3".to_owned().into()))
    );
    assert_eq!(
        eval("[1, 2, 3].fill().join();"),
        Ok(Value::String(",,".to_owned().into()))
    );
    assert_eq!(
        eval("Array.prototype.fill.call(true, 1) instanceof Boolean;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let object = { length: 4 }; let result = Array.prototype.fill.call(object, 'x', 1, 3); (result === object) + ':' + object.hasOwnProperty('0') + ':' + object[1] + ':' + object[2] + ':' + object.hasOwnProperty('3');"
        ),
        Ok(Value::String("true:false:x:x:false".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let value = {}; let start = Number.MAX_SAFE_INTEGER - 3; let object = { length: Number.MAX_SAFE_INTEGER }; Array.prototype.fill.call(object, value, start, start + 3); (object[start] === value) + ':' + (object[start + 1] === value) + ':' + (object[start + 2] === value);"
        ),
        Ok(Value::String("true:true:true".to_owned().into()))
    );
    assert!(
        eval("let object = { length: 1 }; Object.defineProperty(object, '0', { set: function() { throw new TypeError('nope'); } }); Array.prototype.fill.call(object);").is_err()
    );
    assert_eq!(
        eval("let xs = [1, undefined, 3]; xs.reverse(); xs.length + ':' + xs.join();"),
        Ok(Value::String("3:3,,1".to_owned().into()))
    );
    assert_eq!(
        eval("let xs = [2]; let ys = xs; ys.unshift(1); xs.shift() + ':' + xs.join();"),
        Ok(Value::String("1:2".to_owned().into()))
    );
    assert_eq!(
        eval("let xs = [1]; let ys = xs; ys.push(2); xs.join();"),
        Ok(Value::String("1,2".to_owned().into()))
    );
    assert_eq!(
        eval("let xs = [1]; xs[2] = 3; xs.length + ':' + xs.join();"),
        Ok(Value::String("3:1,,3".to_owned().into()))
    );
    assert_eq!(
        eval("let xs = [1, 2, 3]; xs.length = 1; xs.join();"),
        Ok(Value::String("1".to_owned().into()))
    );
}

#[test]
fn array_mutators_preserve_proxy_set_through_live_array_prototypes() {
    assert_eq!(
        eval(
            "let push = Array.prototype.push; \
             let fill = Array.prototype.fill; \
             let copyWithin = Array.prototype.copyWithin; \
             let hasOwn = Object.prototype.hasOwnProperty; \
             let objectPushLog = '', objectPushBridge = [], objectPush; \
             Object.setPrototypeOf(objectPushBridge, new Proxy({}, { \
                 set: function(target, key, value, receiver) { \
                     objectPushLog += key + ':' + value + ':' + (receiver === objectPush) + '|'; \
                     return true; \
                 } \
             })); \
             objectPush = Object.create(objectPushBridge); \
             let objectPushLength = push.call(objectPush, 7); \
             let objectFillLog = '', objectFillBridge = new Array(1), objectFill; \
             Object.setPrototypeOf(objectFillBridge, new Proxy({}, { \
                 set: function(target, key, value, receiver) { \
                     objectFillLog += key + ':' + value + ':' + (receiver === objectFill) + '|'; \
                     return true; \
                 } \
             })); \
             objectFill = Object.create(objectFillBridge); \
             fill.call(objectFill, 8); \
             let objectCopyLog = '', objectCopyBridge = [], objectCopy; \
             Object.setPrototypeOf(objectCopyBridge, new Proxy({}, { \
                 set: function(target, key, value, receiver) { \
                     objectCopyLog += key + ':' + value + ':' + (receiver === objectCopy) + '|'; \
                     return true; \
                 } \
             })); \
             objectCopy = Object.create(objectCopyBridge); \
             objectCopy.length = 2; \
             Object.defineProperty(objectCopy, '1', { \
                 value: 5, writable: true, enumerable: true, configurable: true \
             }); \
             copyWithin.call(objectCopy, 0, 1, 2); \
             let original = Object.getPrototypeOf(Array.prototype); \
             function observeArray(receiver, method, args) { \
                 let log = ''; \
                 let proxy = new Proxy({}, { \
                     set: function(target, key, value, actualReceiver) { \
                         log += key + ':' + value + ':' + (actualReceiver === receiver) + '|'; \
                         return true; \
                     } \
                 }); \
                 let result; \
                 Object.setPrototypeOf(Array.prototype, proxy); \
                 try { result = method.apply(receiver, args); } \
                 finally { Object.setPrototypeOf(Array.prototype, original); } \
                 return log + ':' + (result === receiver || result) + ':' \
                     + hasOwn.call(receiver, '0') + ':' + receiver.length; \
             } \
             let arrayPush = []; \
             let arrayFill = new Array(1); \
             let arrayCopy = new Array(2); arrayCopy[1] = 5; \
             objectPushLength + ':' + objectPushLog + ':' \
                 + hasOwn.call(objectPush, '0') + ':' + objectPush.length + ';' \
                 + objectFillLog + ':' + hasOwn.call(objectFill, '0') + ';' \
                 + objectCopyLog + ':' + hasOwn.call(objectCopy, '0') + ';' \
                 + observeArray(arrayPush, push, [7]) + ';' \
                 + observeArray(arrayFill, fill, [8]) + ';' \
                 + observeArray(arrayCopy, copyWithin, [0, 1, 2]);"
        ),
        Ok(Value::String(
            "1:0:7:true|:false:1;0:8:true|:false;0:5:true|:false;\
             0:7:true|:1:false:1;0:8:true|:true:false:1;\
             0:5:true|:true:false:2"
                .replace("             ", "")
                .into()
        ))
    );
}

#[test]
fn array_mutators_dispatch_proxy_delete_internal_method() {
    assert_eq!(
        eval(
            "function observe(method, target, args) { \
                 let log = ''; \
                 let proxy = new Proxy(target, { \
                     deleteProperty: function(actualTarget, key) { \
                         log += key + '|'; \
                         return Reflect.deleteProperty(actualTarget, key); \
                     } \
                 }); \
                 method.apply(proxy, args); \
                 return log; \
             } \
             observe(Array.prototype.pop, { 0: 1, length: 1 }, []) + ':' \
                 + observe(Array.prototype.shift, { 0: 1, length: 1 }, []) + ':' \
                 + observe(Array.prototype.unshift, { 1: 2, length: 1 }, [3]) + ':' \
                 + observe(Array.prototype.copyWithin, { 0: 1, length: 2 }, [0, 1, 2]);"
        ),
        Ok(Value::String("0|:0|:1|:0|".to_owned().into()))
    );
}
