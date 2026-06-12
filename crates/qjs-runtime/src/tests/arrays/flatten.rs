use crate::{Value, eval};

#[test]
fn evaluates_array_flat_default_depth() {
    assert_eq!(
        eval("[1, [2, 3], 4].flat().join();"),
        Ok(Value::String("1,2,3,4".to_owned()))
    );
    assert_eq!(
        eval(
            "let flat = [1, [2, [3]]].flat(); flat.length + ':' + flat[1] + ':' + Array.isArray(flat[2]) + ':' + flat[2][0];"
        ),
        Ok(Value::String("3:2:true:3".to_owned()))
    );
}

#[test]
fn evaluates_array_flat_explicit_depth() {
    assert_eq!(
        eval("[1, [2, [3, [4]]]].flat(2).join();"),
        Ok(Value::String("1,2,3,4".to_owned()))
    );
    assert_eq!(
        eval(
            "let flat = [1, [2, [3, [4]]]].flat(0); flat.length + ':' + Array.isArray(flat[1]) + ':' + flat[1][0];"
        ),
        Ok(Value::String("2:true:2".to_owned()))
    );
    assert_eq!(
        eval("[1, [2, [3, [4]]]].flat(Infinity).join();"),
        Ok(Value::String("1,2,3,4".to_owned()))
    );
}

#[test]
fn evaluates_array_flat_depth_coercion_and_values() {
    assert_eq!(
        eval("[1, [2]].flat('1').join();"),
        Ok(Value::String("1,2".to_owned()))
    );
    assert_eq!(
        eval("[1, [2]].flat('x').join('|');"),
        Ok(Value::String("1|2".to_owned()))
    );
    assert_eq!(
        eval(
            "[1, [null, undefined]].flat().length + ':' + ([1, [null, undefined]].flat()[1] === null) + ':' + ([1, [null, undefined]].flat()[2] === undefined);"
        ),
        Ok(Value::String("3:true:true".to_owned()))
    );
    assert_eq!(
        eval(
            "Array.prototype.flat.call(true).length + ':' + Array.prototype.flat.call(false).length;"
        ),
        Ok(Value::String("0:0".to_owned()))
    );
}

#[test]
fn evaluates_array_flat_species_constructor_validation() {
    assert_eq!(
        eval(
            "let values = [null, 1, 'string', true]; let result = []; for (let index = 0; index < values.length; index = index + 1) { let array = []; array.constructor = values[index]; try { array.flat(); result.push(false); } catch (error) { result.push(error instanceof TypeError); } } result.join();"
        ),
        Ok(Value::String("true,true,true,true".to_owned()))
    );
    assert_eq!(
        eval(
            "let values = [null, 1, 'string', true]; let result = []; for (let index = 0; index < values.length; index = index + 1) { let array = []; array.constructor = values[index]; try { array.flatMap(function(value) { return [value]; }); result.push(false); } catch (error) { result.push(error instanceof TypeError); } } result.join();"
        ),
        Ok(Value::String("true,true,true,true".to_owned()))
    );
}

#[test]
fn evaluates_array_flat_species_result_write_failures() {
    assert_eq!(
        eval(
            "function C() { this.length = 0; Object.preventExtensions(this); } \
             let a = [1]; a.constructor = {}; a.constructor[Symbol.species] = C; \
             let caught = false; try { a.flat(); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "function C() { Object.defineProperty(this, '0', { set: function() {}, configurable: false }); } \
             let a = [[1]]; a.constructor = {}; a.constructor[Symbol.species] = C; \
             let caught = false; try { a.flat(); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn evaluates_array_flat_proxy_access_order() {
    assert_eq!(
        eval(
            "let getCalls = []; let hasCalls = []; \
             let handler = { \
               get: function(target, key) { getCalls[getCalls.length] = key; return target[key]; }, \
               has: function(target, key) { hasCalls[hasCalls.length] = key; return Reflect.has(target, key); } \
             }; \
             let tier2 = new Proxy([4, 3], handler); \
             let tier1 = new Proxy([2, [3, [4, 2], 2], 5, tier2, 6], handler); \
             Array.prototype.flat.call(tier1, 3); \
             getCalls.join(',') + '|' + hasCalls.join(',');"
        ),
        Ok(Value::String(
            "length,constructor,0,1,2,3,length,0,1,4|0,1,2,3,0,1,4".to_owned(),
        ))
    );
}
