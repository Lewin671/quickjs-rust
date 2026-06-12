use crate::{Value, eval};

#[test]
fn evaluates_array_builtins() {
    assert_eq!(
        eval("typeof Array;"),
        Ok(Value::String("function".to_owned()))
    );
    assert_eq!(eval("Array.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Array.from.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Array.isArray.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Array.of.length;"), Ok(Value::Number(0.0)));
    assert_eq!(eval("Array.prototype.length;"), Ok(Value::Number(0.0)));
    assert_eq!(eval("Array.prototype.at.length;"), Ok(Value::Number(1.0)));
    assert_eq!(
        eval("Array.prototype.concat.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("Array.prototype.copyWithin.length;"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval("Array.prototype.every.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(eval("Array.prototype.fill.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Array.prototype.flat.length;"), Ok(Value::Number(0.0)));
    assert_eq!(
        eval("Array.prototype.flatMap.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("Array.prototype.filter.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(eval("Array.prototype.find.length;"), Ok(Value::Number(1.0)));
    assert_eq!(
        eval("Array.prototype.findIndex.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("Array.prototype.findLast.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("Array.prototype.findLastIndex.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("Array.prototype.forEach.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("Array.prototype.includes.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("Array.prototype.indexOf.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("Array.prototype.lastIndexOf.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(eval("Array.prototype.map.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Array.prototype.join.length;"), Ok(Value::Number(1.0)));
    assert_eq!(
        eval("Array.prototype.slice.length;"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(eval("Array.prototype.some.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Array.prototype.sort.length;"), Ok(Value::Number(1.0)));
    assert_eq!(
        eval("Array.prototype.splice.length;"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(eval("Array.prototype.pop.length;"), Ok(Value::Number(0.0)));
    assert_eq!(eval("Array.prototype.push.length;"), Ok(Value::Number(1.0)));
    assert_eq!(
        eval("Array.prototype.reduce.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("Array.prototype.reduceRight.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("Array.prototype.shift.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("Array.prototype.reverse.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("Array.prototype.unshift.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("Array.prototype.toString.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("Array.prototype.toLocaleString.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval(
            "let d = Object.getOwnPropertyDescriptor(Array.prototype, 'toLocaleString'); (d.value === Array.prototype.toLocaleString) + ':' + d.writable + ':' + d.enumerable + ':' + d.configurable;"
        ),
        Ok(Value::String("true:true:false:true".to_owned()))
    );
    assert_eq!(
        eval("[1, 'x', true].toLocaleString();"),
        Ok(Value::String("1,x,true".to_owned()))
    );
    assert_eq!(
        eval(
            "let calls = 0; let item = { toLocaleString: function() { calls++; return 'item'; } }; \
             [undefined, item, null, item].toLocaleString(); calls;"
        ),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval(
            "'use strict'; Boolean.prototype.toString = function() { return typeof this; }; \
             [true, false].toLocaleString();"
        ),
        Ok(Value::String("boolean,boolean".to_owned()))
    );
    assert_eq!(
        eval(
            "let calls = 0; let item = { toLocaleString: function() { calls++; return 'proto'; } }; \
             Array.prototype[1] = item; let xs = [item]; xs.length = 2; \
             let result = xs.toLocaleString() + ':' + calls; delete Array.prototype[1]; result;"
        ),
        Ok(Value::String("proto,proto:2".to_owned()))
    );
    assert_eq!(eval("Array().length;"), Ok(Value::Number(0.0)));
    assert_eq!(eval("Array(1, 2)[1];"), Ok(Value::Number(2.0)));
    assert_eq!(
        eval("let array = new Array('x'); array[0];"),
        Ok(Value::String("x".to_owned()))
    );
    assert_eq!(eval("Array.isArray([]);"), Ok(Value::Boolean(true)));
    assert_eq!(
        eval("Array.isArray(Array.prototype);"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(eval("Array.isArray({});"), Ok(Value::Boolean(false)));
    assert_eq!(eval("Array.isArray('abc');"), Ok(Value::Boolean(false)));
    assert_eq!(
        eval(
            "let objectProxy = new Proxy({}, {}); \
             let arrayProxy = new Proxy([], {}); \
             let arrayProxyProxy = new Proxy(arrayProxy, {}); \
             Array.isArray(objectProxy) + ':' + Array.isArray(arrayProxy) + ':' + Array.isArray(arrayProxyProxy);"
        ),
        Ok(Value::String("false:true:true".to_owned()))
    );
    assert_eq!(
        eval(
            "let handle = Proxy.revocable([], {}); handle.revoke(); \
             let caught = false; try { Array.isArray(handle.proxy); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Array.prototype.constructor === Array;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(eval("[] instanceof Array;"), Ok(Value::Boolean(true)));
    assert_eq!(
        eval("Array.prototype.isPrototypeOf([]);"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Object.getPrototypeOf([]) === Array.prototype;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(eval("Array(3).length;"), Ok(Value::Number(3.0)));
    assert_eq!(eval("new Array(3).length;"), Ok(Value::Number(3.0)));
    assert_eq!(eval("0 in new Array(3);"), Ok(Value::Boolean(false)));
    assert_eq!(
        eval("let array = new Array(); array.extra = 7; array.extra;"),
        Ok(Value::Number(7.0))
    );
    assert!(eval("Array(-1);").is_err());
    assert!(eval("Array(1.5);").is_err());
}

#[test]
fn array_prototype_is_array_exotic_object() {
    assert_eq!(
        eval(
            "let d = Object.getOwnPropertyDescriptor(Array.prototype, 'length'); \
             Array.prototype[2] = 42; \
             d.writable + ':' + d.enumerable + ':' + d.configurable + ':' + \
             Array.prototype.length + ':' + Array.prototype[2] + ':' + \
             Object.prototype.toString.call(Array.prototype);"
        ),
        Ok(Value::String(
            "true:false:false:3:42:[object Array]".to_owned()
        ))
    );
}

#[test]
fn array_prototype_methods_reject_undefined_this_before_arguments() {
    assert_eq!(
        eval(
            "let concat = Array.prototype.concat; \
             let join = Array.prototype.join; \
             let toString = Array.prototype.toString; \
             let toLocaleString = Array.prototype.toLocaleString; \
             let count = 0; \
             try { concat(); } catch (error) { if (error instanceof TypeError) count++; } \
             try { join({ get toString() { throw new Error('separator'); } }); } catch (error) { if (error instanceof TypeError) count++; } \
             try { toString(); } catch (error) { if (error instanceof TypeError) count++; } \
             try { toLocaleString(); } catch (error) { if (error instanceof TypeError) count++; } \
             count === 4;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn array_prototype_to_string_falls_back_to_intrinsic_object_to_string() {
    assert_eq!(
        eval(
            "delete Object.prototype.toString; \
             let object = Array.prototype.toString.call({ join: null }); \
             let target = []; \
             target.join = undefined; \
             let proxy = new Proxy(target, {}); \
             let array = Array.prototype.toString.call(proxy); \
             let callable = Array.prototype.toString.call(new Proxy(function() {}, {})); \
             object + ':' + array + ':' + callable;"
        ),
        Ok(Value::String(
            "[object Object]:[object Array]:[object Function]".to_owned()
        ))
    );
}

#[test]
fn array_prototype_to_string_reads_join_before_length() {
    assert_eq!(
        eval(
            "let order = []; \
             let object = { \
               get join() { order.push('join'); return null; }, \
               get length() { order.push('length'); return 0; } \
             }; \
             Array.prototype.toString.call(object); \
             order.join(',');"
        ),
        Ok(Value::String("join".to_owned()))
    );
}
