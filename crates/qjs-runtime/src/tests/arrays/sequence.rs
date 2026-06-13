use crate::{Value, eval};

#[test]
fn evaluates_array_sequence_builtins() {
    assert_eq!(
        eval("[1, 'x', true].join();"),
        Ok(Value::String("1,x,true".to_owned()))
    );
    assert_eq!(
        eval("[1, 2, 3].join('|');"),
        Ok(Value::String("1|2|3".to_owned()))
    );
    assert_eq!(
        eval("[1, null, undefined, 4].join('-');"),
        Ok(Value::String("1---4".to_owned()))
    );
    assert_eq!(
        eval("[1, 'x', true].toString();"),
        Ok(Value::String("1,x,true".to_owned()))
    );
    assert_eq!(
        eval("[1, [2, 3], 4].join(';');"),
        Ok(Value::String("1;2,3;4".to_owned()))
    );
    assert_eq!(
        eval("Array.prototype.join.call({ length: 3, 0: 'a', 2: 'c' }, '|');"),
        Ok(Value::String("a||c".to_owned()))
    );
    assert_eq!(
        eval("Array.prototype.join.call('abc', '-');"),
        Ok(Value::String("a-b-c".to_owned()))
    );
    assert_eq!(
        eval(
            "Array.prototype.toString.call({ length: 2, 0: 'x', 1: 'y', join: Array.prototype.join });"
        ),
        Ok(Value::String("x,y".to_owned()))
    );
    assert_eq!(
        eval("Array.prototype.toString.call(true);"),
        Ok(Value::String("[object Boolean]".to_owned()))
    );
    assert_eq!(
        eval("[0, 1, 2, 3, 4].slice(1, 4).join();"),
        Ok(Value::String("1,2,3".to_owned()))
    );
    assert_eq!(
        eval("[0, 1, 2, 3, 4].slice(2).join('|');"),
        Ok(Value::String("2|3|4".to_owned()))
    );
    assert_eq!(
        eval("[0, 1, 2, 3, 4].slice(-3, -1).join();"),
        Ok(Value::String("2,3".to_owned()))
    );
    assert_eq!(
        eval("Array.prototype.slice.call({ length: 5, 0: 0, 1: 1, 2: 2, 4: 4 }, 1, 4).join('|');"),
        Ok(Value::String("1|2|".to_owned()))
    );
    assert_eq!(
        eval("Array.prototype.slice.call('abcd', 1, 3).join('');"),
        Ok(Value::String("bc".to_owned()))
    );
    assert_eq!(
        eval(
            "let a = []; a.constructor = 1; let caught = false; try { a.slice(); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let marker = { ok: true }; let a = []; Object.defineProperty(a, 'constructor', { get: function() { throw marker; } }); let caught = false; try { a.slice(); } catch (error) { caught = error === marker; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let obj = { length: 4294967296 }; let caught = false; try { Array.prototype.slice.call(obj); } catch (error) { caught = error instanceof RangeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let calls = 0; let obj = Object.defineProperty({}, 'length', { get: function() { return 4294967296; }, set: function() { calls = calls + 1; } }); try { Array.prototype.slice.call(obj); } catch (error) {} calls;"
        ),
        Ok(Value::Number(0.0))
    );
    assert_eq!(eval("[0, 1, 2].slice(5).length;"), Ok(Value::Number(0.0)));
    assert_eq!(
        eval("let copy = [1, 2].slice(); Array.isArray(copy) && copy[1] === 2;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let target; function C(length) { this.lengthValue = length; target = this; } let a = [1, 2, 3]; a.constructor = {}; a.constructor[Symbol.species] = C; let out = a.slice(1, 3); out === target && out.lengthValue === 2 && out[0] === 2 && out[1] === 3 && !Object.prototype.hasOwnProperty.call(out, 'length');"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let calls = 0; function C(length) { calls = calls + 1; this.lengthValue = length; } let a = [1]; a.constructor = {}; a.constructor[Symbol.species] = C; let out = a.slice(1); calls + ':' + out.lengthValue;"
        ),
        Ok(Value::String("1:0".to_owned()))
    );
    assert_eq!(
        eval(
            "let target = {}; Object.preventExtensions(target); function C() { return target; } let a = [1]; a.constructor = {}; a.constructor[Symbol.species] = C; let caught = false; try { a.slice(); } catch (error) { caught = error.constructor === TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let target = {}; Object.defineProperty(target, '0', { value: 1, configurable: false }); function C() { return target; } let a = [2]; a.constructor = {}; a.constructor[Symbol.species] = C; let caught = false; try { a.slice(); } catch (error) { caught = error.constructor === TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("[0].concat([1, 2], 3, [4]).join();"),
        Ok(Value::String("0,1,2,3,4".to_owned()))
    );
    assert_eq!(
        eval("[].concat([0, 1], [2, 3]).length;"),
        Ok(Value::Number(4.0))
    );
    assert_eq!(eval("[0].concat('x', true)[2];"), Ok(Value::Boolean(true)));
    assert_eq!(
        eval("Array.prototype.concat.call(true)[0] instanceof Boolean;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let a = []; a.constructor = 1; let caught = false; try { a.concat(); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let marker = { ok: true }; let a = []; Object.defineProperty(a, 'constructor', { get: function() { throw marker; } }); let caught = false; try { a.concat(); } catch (error) { caught = error === marker; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let calls = 0; let lengthArg = -1; let instance = []; function C(length) { calls = calls + 1; lengthArg = length; return instance; } let a = []; a.constructor = {}; a.constructor[Symbol.species] = C; let out = a.concat(); calls + ':' + lengthArg + ':' + (out === instance);"
        ),
        Ok(Value::String("1:0:true".to_owned()))
    );
    assert_eq!(
        eval(
            "let calls = 0; function C() {} Object.defineProperty(C, '__quickjsRustCrossRealmArray', { value: true }); Object.defineProperty(C, Symbol.species, { get: function() { calls = calls + 1; } }); let a = []; a.constructor = C; let out = a.concat(); Array.isArray(out) + ':' + calls + ':' + out.length;"
        ),
        Ok(Value::String("true:0:0".to_owned()))
    );
    assert_eq!(
        eval(
            "let a = []; a.constructor = {}; a.constructor[Symbol.species] = parseInt; let caught = false; try { a.concat(); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "function C() { Object.preventExtensions(this); } let a = []; a.constructor = {}; a.constructor[Symbol.species] = C; let caught = false; try { a.concat(1); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "function C() { Object.defineProperty(this, '0', { set: function() {}, configurable: false }); } let a = []; a.constructor = {}; a.constructor[Symbol.species] = C; let caught = false; try { a.concat(1); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "Array.prototype[1] = 1; let x = [0]; x.length = 2; let out = x.concat(); out[0] + ':' + out[1] + ':' + out.hasOwnProperty('1');"
        ),
        Ok(Value::String("0:1:true".to_owned()))
    );
    assert_eq!(
        eval(
            "let a = [0]; a.length = 3; let b = a.concat(); b.length + ':' + b.hasOwnProperty('1') + ':' + (b[1] === undefined);"
        ),
        Ok(Value::String("3:false:true".to_owned()))
    );
    assert_eq!(
        eval(
            "let a = [1, 2]; a[Symbol.isConcatSpreadable] = false; let out = [0].concat(a); out.length + ':' + (out[1] === a);"
        ),
        Ok(Value::String("2:true".to_owned()))
    );
    assert_eq!(
        eval("let a = [1, 2]; a[Symbol.isConcatSpreadable] = undefined; [0].concat(a).join();"),
        Ok(Value::String("0,1,2".to_owned()))
    );
    assert_eq!(
        eval(
            "let item = { 0: 'a', 2: 'c', length: 3 }; item[Symbol.isConcatSpreadable] = true; let out = [0].concat(item); out.length + ':' + out[1] + ':' + out.hasOwnProperty('2') + ':' + out[3];"
        ),
        Ok(Value::String("4:a:false:c".to_owned()))
    );
    assert_eq!(
        eval(
            "let item = { length: 4000 }; item[Symbol.isConcatSpreadable] = true; let out = [].concat(item); out.length + ':' + out.hasOwnProperty('0') + ':' + out.hasOwnProperty('3999') + ':' + (out[3999] === undefined);"
        ),
        Ok(Value::String("4000:false:false:true".to_owned()))
    );
    assert_eq!(
        eval(
            "let item = new Uint8Array(1); Object.defineProperty(item, 'length', { value: 4 }); item[Symbol.isConcatSpreadable] = true; let out = [].concat(item); out.length + ':' + out[0] + ':' + out.hasOwnProperty('1') + ':' + (out[1] === undefined);"
        ),
        Ok(Value::String("4:0:false:true".to_owned()))
    );
    assert_eq!(
        eval(
            "let item = new Uint16Array(4000); for (let i = 0; i < item.length; i++) item[i] = i; item[Symbol.isConcatSpreadable] = true; let out = [].concat(item); out.length + ':' + out[0] + ':' + out[3999];"
        ),
        Ok(Value::String("4000:0:3999".to_owned()))
    );
    assert_eq!(
        eval(
            "let proto = { 2: 'p' }; let item = Object.create(proto); item.length = 4; item[Symbol.isConcatSpreadable] = true; let out = [].concat(item); out.length + ':' + out.hasOwnProperty('2') + ':' + out[2];"
        ),
        Ok(Value::String("4:true:p".to_owned()))
    );
    assert_eq!(
        eval(
            "let args = (function(a, b, c) { return arguments; })(1, 2, 3); args[Symbol.isConcatSpreadable] = true; [].concat(args, args).join('|');"
        ),
        Ok(Value::String("1|2|3|1|2|3".to_owned()))
    );
    assert_eq!(
        eval(
            "let args = (function(a) { return arguments; })(1, 2, 3); delete args[1]; args[Symbol.isConcatSpreadable] = true; let out = [].concat(args, args); out.join('|') + ':' + out.hasOwnProperty('1') + ':' + out.hasOwnProperty('4');"
        ),
        Ok(Value::String("1||3|1||3:false:false".to_owned()))
    );
    assert_eq!(
        eval(
            "let marker = { ok: true }; let item = {}; Object.defineProperty(item, Symbol.isConcatSpreadable, { get: function() { throw marker; } }); let caught = false; try { [].concat(item); } catch (error) { caught = error === marker; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let item = { length: Number.MAX_SAFE_INTEGER }; item[Symbol.isConcatSpreadable] = true; let caught = false; try { [1].concat(item); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let xs = [1, 2, 3, 4, 5]; let result = xs.copyWithin(0, 3); result === xs && xs.join();"
        ),
        Ok(Value::String("4,5,3,4,5".to_owned()))
    );
    assert_eq!(
        eval("[1, 2, 3, 4, 5].copyWithin(1, 3, 4).join();"),
        Ok(Value::String("1,4,3,4,5".to_owned()))
    );
    assert_eq!(
        eval("[1, 2, 3, 4, 5].copyWithin(-2, 0, 2).join();"),
        Ok(Value::String("1,2,3,1,2".to_owned()))
    );
    assert_eq!(
        eval("[1, 2, 3, 4].copyWithin(1, 0, 3).join();"),
        Ok(Value::String("1,1,2,3".to_owned()))
    );
    assert_eq!(
        eval(
            "let o = { length: 4, 0: 'a', 2: 'c', 3: 'd' }; Array.prototype.copyWithin.call(o, 1, 2); o[0] + ':' + o[1] + ':' + o[2] + ':' + o[3];"
        ),
        Ok(Value::String("a:c:d:d".to_owned()))
    );
    assert_eq!(
        eval(
            "let xs = [1, , 3]; xs.copyWithin(0, 1, 2); xs.hasOwnProperty('0') + ':' + (xs[0] === undefined);"
        ),
        Ok(Value::String("false:true".to_owned()))
    );
    assert_eq!(
        eval("Array.prototype.copyWithin.call(true) instanceof Boolean;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let xs = [0, 1, 2, 3]; xs.copyWithin(0, { valueOf: function() { xs.length = 2; return 3; } }); xs.length + ':' + xs.hasOwnProperty('0') + ':' + xs[1];"
        ),
        Ok(Value::String("2:false:1".to_owned()))
    );
    assert_eq!(
        eval(
            "let proto = { 3: 9 }; let xs = [0, 1, 2, 3]; Object.setPrototypeOf(xs, proto); Array.prototype.copyWithin.call(xs, 0, { valueOf: function() { xs.length = 2; return 3; } }); xs.length + ':' + xs[0] + ':' + xs[1];"
        ),
        Ok(Value::String("2:9:1".to_owned()))
    );
    assert_eq!(
        eval(
            "let proto = [0, 1, 2, 3, 4]; let xs = [0, 1, 2, 3, 4]; Object.setPrototypeOf(xs, proto); xs.copyWithin(0, { valueOf: function() { xs.length = 2; return 3; } }); xs.length + ':' + xs[0] + ':' + xs[1];"
        ),
        Ok(Value::String("2:3:4".to_owned()))
    );
    assert_eq!(
        eval(
            "let o = { length: 43 }; Object.defineProperty(o, '42', { configurable: false, writable: true }); let caught = false; try { Array.prototype.copyWithin.call(o, 42, 0); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let marker = { ok: true }; let o = { 0: true, length: 43 }; Object.defineProperty(o, '42', { set: function() { throw marker; } }); let caught = false; try { Array.prototype.copyWithin.call(o, 42, 0); } catch (error) { caught = error === marker; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let iterator = ['a', 'b'].entries(); let first = iterator.next(); let second = iterator.next(); let last = iterator.next(); first.done + ':' + first.value[0] + ':' + first.value[1] + '|' + second.value[0] + ':' + second.value[1] + '|' + last.done + ':' + (last.value === undefined);"
        ),
        Ok(Value::String("false:0:a|1:b|true:true".to_owned()))
    );
    assert_eq!(
        eval(
            "let array = []; let iterator = array.entries(); array.push('a'); let first = iterator.next(); let done = iterator.next(); array.push('b'); let stillDone = iterator.next(); first.value[1] + ':' + done.done + ':' + stillDone.done;"
        ),
        Ok(Value::String("a:true:true".to_owned()))
    );
    assert_eq!(
        eval(
            "let iterator = ['a', 'b'].keys(); let first = iterator.next(); let second = iterator.next(); let last = iterator.next(); first.value + ':' + first.done + '|' + second.value + ':' + second.done + '|' + (last.value === undefined) + ':' + last.done;"
        ),
        Ok(Value::String("0:false|1:false|true:true".to_owned()))
    );
    assert_eq!(
        eval(
            "let iterator = ['a', 'b'].values(); let first = iterator.next(); let second = iterator.next(); let last = iterator.next(); first.value + ':' + first.done + '|' + second.value + ':' + second.done + '|' + (last.value === undefined) + ':' + last.done;"
        ),
        Ok(Value::String("a:false|b:false|true:true".to_owned()))
    );
    assert_eq!(
        eval(
            "let array = []; let keys = array.keys(); let values = array.values(); array.push('a'); let key = keys.next(); let value = values.next(); key.value + ':' + value.value + ':' + keys.next().done + ':' + values.next().done;"
        ),
        Ok(Value::String("0:a:true:true".to_owned()))
    );
    assert!(eval("Array.prototype.entries.call(undefined);").is_err());
    assert!(eval("Array.prototype.entries.call(null);").is_err());
    assert!(eval("Array.prototype.keys.call(undefined);").is_err());
    assert!(eval("Array.prototype.values.call(null);").is_err());
}

#[test]
fn exposes_array_unscopables() {
    assert_eq!(
        eval("let u = Array.prototype[Symbol.unscopables]; Object.getPrototypeOf(u) === null;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let d = Object.getOwnPropertyDescriptor(Array.prototype, Symbol.unscopables); d.writable + ':' + d.enumerable + ':' + d.configurable;"
        ),
        Ok(Value::String("false:false:true".to_owned()))
    );
    assert_eq!(
        eval(
            "let u = Array.prototype[Symbol.unscopables]; ['copyWithin', 'entries', 'fill', 'find', 'findIndex', 'findLast', 'findLastIndex', 'flat', 'flatMap', 'includes', 'keys', 'toReversed', 'toSorted', 'toSpliced', 'values'].every(function(key) { let d = Object.getOwnPropertyDescriptor(u, key); return d.value === true && d.writable && d.enumerable && d.configurable; });"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Object.prototype.hasOwnProperty.call(Array.prototype[Symbol.unscopables], 'with');"),
        Ok(Value::Boolean(false))
    );
}

#[test]
fn evaluates_array_to_reversed() {
    assert_eq!(
        eval(
            "let xs = [1, 2, 3]; let out = xs.toReversed(); out.join() + ':' + xs.join() + ':' + (out === xs);"
        ),
        Ok(Value::String("3,2,1:1,2,3:false".to_owned()))
    );
    assert_eq!(eval("[].toReversed().length;"), Ok(Value::Number(0.0)));
    assert_eq!(eval("[7].toReversed()[0];"), Ok(Value::Number(7.0)));
    assert_eq!(
        eval("Array.prototype.toReversed.call({ length: 3, 0: 'a', 2: 'c' }).join('|');"),
        Ok(Value::String("c||a".to_owned()))
    );
    assert_eq!(
        eval("Array.prototype.toReversed.call('abc').join('');"),
        Ok(Value::String("cba".to_owned()))
    );
    assert_eq!(
        eval("Array.prototype.toReversed.length;"),
        Ok(Value::Number(0.0))
    );
    assert!(eval("Array.prototype.toReversed.call(null);").is_err());
}

#[test]
fn evaluates_array_to_spliced() {
    assert_eq!(
        eval(
            "let xs = [1, 2, 3, 4]; let out = xs.toSpliced(1, 2, 'a', 'b'); out.join() + ':' + xs.join() + ':' + (out === xs);"
        ),
        Ok(Value::String("1,a,b,4:1,2,3,4:false".to_owned()))
    );
    assert_eq!(
        eval("[1, 2, 3].toSpliced(-1, 1, 9).join();"),
        Ok(Value::String("1,2,9".to_owned()))
    );
    assert_eq!(
        eval("[1, 2, 3].toSpliced(1).join();"),
        Ok(Value::String("1".to_owned()))
    );
    assert_eq!(
        eval("[1, 2, 3].toSpliced(1, undefined, 9).join();"),
        Ok(Value::String("1,9,2,3".to_owned()))
    );
    assert_eq!(
        eval("[1, 2, 3].toSpliced(8, 1, 4).join();"),
        Ok(Value::String("1,2,3,4".to_owned()))
    );
    assert_eq!(
        eval("Array.prototype.toSpliced.call({ length: 3, 0: 'a', 2: 'c' }, 1, 1, 'b').join('|');"),
        Ok(Value::String("a|b|c".to_owned()))
    );
    assert_eq!(
        eval("Array.prototype.toSpliced.call('abc', 1, 1, 'x').join('');"),
        Ok(Value::String("axc".to_owned()))
    );
    assert_eq!(
        eval("Array.prototype.toSpliced.length;"),
        Ok(Value::Number(2.0))
    );
    assert!(eval("Array.prototype.toSpliced.call(null, 0, 0);").is_err());
}

#[test]
fn evaluates_array_with() {
    assert_eq!(
        eval(
            "let xs = [1, 2, 3]; let out = xs.with(1, 9); out.join() + ':' + xs.join() + ':' + (out === xs);"
        ),
        Ok(Value::String("1,9,3:1,2,3:false".to_owned()))
    );
    assert_eq!(
        eval("[1, 2, 3].with(-1, 9).join();"),
        Ok(Value::String("1,2,9".to_owned()))
    );
    assert_eq!(
        eval("[1, 2, 3].with(undefined, 9).join();"),
        Ok(Value::String("9,2,3".to_owned()))
    );
    assert_eq!(
        eval("[1, 2, 3].with(1).join();"),
        Ok(Value::String("1,,3".to_owned()))
    );
    assert_eq!(
        eval("Array.prototype.with.call({ length: 3, 0: 'a', 2: 'c' }, 1, 'b').join('|');"),
        Ok(Value::String("a|b|c".to_owned()))
    );
    assert_eq!(
        eval("Array.prototype.with.call('abc', -2, 'x').join('');"),
        Ok(Value::String("axc".to_owned()))
    );
    assert_eq!(eval("Array.prototype.with.length;"), Ok(Value::Number(2.0)));
    assert!(eval("[].with(0, 1);").is_err());
    assert!(eval("[1].with(1, 2);").is_err());
    assert!(eval("Array.prototype.with.call(null, 0, 1);").is_err());
}
