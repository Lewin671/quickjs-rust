use crate::{Value, eval};

#[test]
fn evaluates_array_splice_deletion() {
    assert_eq!(
        eval(
            "let xs = [1, 2, 3, 4]; let removed = xs.splice(1, 2); removed.join() + ':' + xs.join();"
        ),
        Ok(Value::String("2,3:1,4".to_owned()))
    );
    assert_eq!(
        eval(
            "let xs = [1, 2, 3, 4]; let removed = xs.splice(-2, 1); removed.join() + ':' + xs.join();"
        ),
        Ok(Value::String("3:1,2,4".to_owned()))
    );
    assert_eq!(
        eval("let xs = [1, 2, 3]; let removed = xs.splice(1); removed.join() + ':' + xs.join();"),
        Ok(Value::String("2,3:1".to_owned()))
    );
}

#[test]
fn evaluates_array_splice_insertion_and_replacement() {
    assert_eq!(
        eval(
            "let xs = [1, 4]; let removed = xs.splice(1, 0, 2, 3); removed.length + ':' + xs.join();"
        ),
        Ok(Value::String("0:1,2,3,4".to_owned()))
    );
    assert_eq!(
        eval(
            "let xs = [1, 2, 5]; let removed = xs.splice(1, 1, 3, 4); removed.join() + ':' + xs.join();"
        ),
        Ok(Value::String("2:1,3,4,5".to_owned()))
    );
    assert_eq!(
        eval(
            "let xs = [1, undefined, 3]; let removed = xs.splice(1, 1, 2); (removed[0] === undefined) + ':' + xs.join();"
        ),
        Ok(Value::String("true:1,2,3".to_owned()))
    );
}

#[test]
fn evaluates_array_splice_bounds() {
    assert_eq!(
        eval(
            "let xs = [1, 2]; let removed = xs.splice(10, 1, 3); removed.length + ':' + xs.join();"
        ),
        Ok(Value::String("0:1,2,3".to_owned()))
    );
    assert_eq!(
        eval(
            "let xs = [1, 2]; let removed = xs.splice(0, -1, 0); removed.length + ':' + xs.join();"
        ),
        Ok(Value::String("0:0,1,2".to_owned()))
    );
}

#[test]
fn evaluates_array_splice_generic_receivers() {
    assert_eq!(
        eval(
            "let object = {0: 0, 1: 1, 2: 2, 3: 3, length: 4}; let removed = Array.prototype.splice.call(object, 0, 3, 4, 5); removed.join() + ':' + removed.length + ':' + object.length + ':' + object[0] + ':' + object[1] + ':' + object[2] + ':' + object[3];"
        ),
        Ok(Value::String("0,1,2:3:3:4:5:3:undefined".to_owned()))
    );
    assert_eq!(
        eval(
            "Array.prototype.splice.call(true).length + ':' + Array.prototype.splice.call(false).length;"
        ),
        Ok(Value::String("0:0".to_owned()))
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
        Ok(Value::String("true,true,true,true".to_owned()))
    );
    assert_eq!(
        eval(
            "let marker = { ok: true }; let array = []; Object.defineProperty(array, 'constructor', { get: function() { throw marker; } }); let caught = false; try { array.splice(); } catch (error) { caught = error === marker; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
}
