use crate::{Value, eval};

#[test]
fn evaluates_array_splice_deletion() {
    assert_eq!(
        eval(
            "let xs = [1, 2, 3, 4]; let removed = xs.splice(1, 2); removed.join() + ':' + xs.join();"
        ),
        Ok(Value::String("2,3:1,4".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let xs = [1, 2, 3, 4]; let removed = xs.splice(-2, 1); removed.join() + ':' + xs.join();"
        ),
        Ok(Value::String("3:1,2,4".to_owned().into()))
    );
    assert_eq!(
        eval("let xs = [1, 2, 3]; let removed = xs.splice(1); removed.join() + ':' + xs.join();"),
        Ok(Value::String("2,3:1".to_owned().into()))
    );
}

#[test]
fn evaluates_array_splice_insertion_and_replacement() {
    assert_eq!(
        eval(
            "let xs = [1, 4]; let removed = xs.splice(1, 0, 2, 3); removed.length + ':' + xs.join();"
        ),
        Ok(Value::String("0:1,2,3,4".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let xs = [1, 2, 5]; let removed = xs.splice(1, 1, 3, 4); removed.join() + ':' + xs.join();"
        ),
        Ok(Value::String("2:1,3,4,5".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let xs = [1, undefined, 3]; let removed = xs.splice(1, 1, 2); (removed[0] === undefined) + ':' + xs.join();"
        ),
        Ok(Value::String("true:1,2,3".to_owned().into()))
    );
}

#[test]
fn evaluates_array_splice_bounds() {
    assert_eq!(
        eval(
            "let xs = [1, 2]; let removed = xs.splice(10, 1, 3); removed.length + ':' + xs.join();"
        ),
        Ok(Value::String("0:1,2,3".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let xs = [1, 2]; let removed = xs.splice(0, -1, 0); removed.length + ':' + xs.join();"
        ),
        Ok(Value::String("0:0,1,2".to_owned().into()))
    );
}

#[test]
fn evaluates_array_splice_generic_receivers() {
    assert_eq!(
        eval(
            "let object = {0: 0, 1: 1, 2: 2, 3: 3, length: 4}; let removed = Array.prototype.splice.call(object, 0, 3, 4, 5); removed.join() + ':' + removed.length + ':' + object.length + ':' + object[0] + ':' + object[1] + ':' + object[2] + ':' + object[3];"
        ),
        Ok(Value::String("0,1,2:3:3:4:5:3:undefined".to_owned().into()))
    );
    assert_eq!(
        eval(
            "Array.prototype.splice.call(true).length + ':' + Array.prototype.splice.call(false).length;"
        ),
        Ok(Value::String("0:0".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let object = Object.defineProperty({}, 'length', { get: function() { return Math.pow(2, 32); }, set: function() { throw 'length should not be set'; } }); let caught = false; try { Array.prototype.splice.call(object, 0); } catch (error) { caught = error instanceof RangeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn evaluates_array_splice_species_constructor_validation() {
    assert_eq!(
        eval(
            "let values = [null, 1, 'string', true]; let result = []; for (let index = 0; index < values.length; index = index + 1) { let array = []; array.constructor = values[index]; try { array.splice(); result.push(false); } catch (error) { result.push(error instanceof TypeError); } } result.join();"
        ),
        Ok(Value::String("true,true,true,true".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let marker = { ok: true }; let array = []; Object.defineProperty(array, 'constructor', { get: function() { throw marker; } }); let caught = false; try { array.splice(); } catch (error) { caught = error === marker; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let target; function C(length) { this.lengthValue = length; target = this; } let array = [1, 2, 3, 4]; array.constructor = {}; array.constructor[Symbol.species] = C; let removed = array.splice(1, 2); removed === target && removed.lengthValue === 2 && removed[0] === 2 && removed[1] === 3 && removed.length === 2 && array.join() === '1,4';"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let calls = 0; function C(length) { calls = calls + 1; this.lengthValue = length; } let array = [1]; array.constructor = {}; array.constructor[Symbol.species] = C; let removed = array.splice(0, 0); calls + ':' + removed.lengthValue + ':' + removed.length;"
        ),
        Ok(Value::String("1:0:0".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let target = {}; Object.preventExtensions(target); function C() { return target; } let array = [1]; array.constructor = {}; array.constructor[Symbol.species] = C; let caught = false; try { array.splice(0); } catch (error) { caught = error.constructor === TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let target = {}; Object.defineProperty(target, '0', { value: 1, configurable: false }); function C() { return target; } let array = [2]; array.constructor = {}; array.constructor[Symbol.species] = C; let caught = false; try { array.splice(0); } catch (error) { caught = error.constructor === TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let log = []; let array = [0, 1]; array.constructor = {}; array.constructor[Symbol.species] = function(length) { return new Proxy(new Array(length), new Proxy({}, { get: function(target, key) { log.push(key); } })); }; array.splice(0); log.join('|');"
        ),
        Ok(Value::String(
            "defineProperty|defineProperty|set|getOwnPropertyDescriptor|defineProperty"
                .to_owned()
                .into()
        ))
    );
}
