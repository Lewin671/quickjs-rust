use crate::{Value, eval};

#[test]
fn evaluates_array_flat_map_basic_mapping_and_flattening() {
    assert_eq!(
        eval("[1, 2, 3].flatMap(function(value) { return [value, value * 2]; }).join();"),
        Ok(Value::String("1,2,2,4,3,6".to_owned().into()))
    );
    assert_eq!(
        eval("[1, 2].flatMap(function(value) { return [[value]]; })[0][0];"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval(
            "let called = 0; let out = Array.prototype.flatMap.call(true, function() { called = called + 1; }); out.length + ':' + called;"
        ),
        Ok(Value::String("0:0".to_owned().into()))
    );
}

#[test]
fn evaluates_array_flat_map_callback_arguments_and_this_arg() {
    assert_eq!(
        eval(
            "let source = [10, 20]; let seen = ''; let out = source.flatMap(function(value, index, array) { seen = seen + value + ':' + index + ':' + (array === source) + ':' + this.offset + ';'; return [value + index + this.offset]; }, { offset: 3 }); seen + '|' + out.join();"
        ),
        Ok(Value::String(
            "10:0:true:3;20:1:true:3;|13,24".to_owned().into()
        ))
    );
}

#[test]
fn skips_missing_flattened_indexes_but_reads_inherited_indexes() {
    assert_eq!(
        eval(
            "let calls = []; let out = [1, , 3].flatMap(function(value, index) { calls.push(index); return [value]; }); out.join() + ':' + calls.join();"
        ),
        Ok(Value::String("1,3:0,2".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let proto = []; proto[1] = 9; let xs = [1, , 3]; Object.setPrototypeOf(xs, proto); xs.flat().join();"
        ),
        Ok(Value::String("1,9,3".to_owned().into()))
    );
}

#[test]
fn rejects_array_flat_map_non_callable_callback() {
    assert!(eval("[1].flatMap(null);").is_err());
}

#[test]
fn evaluates_array_flat_map_species_result_object() {
    assert_eq!(
        eval(
            "let calls = 0; let lengthArg = -1; let instance = {}; \
             function C(length) { calls = calls + 1; lengthArg = length; return instance; } \
             let a = [[1], [2]]; a.constructor = {}; a.constructor[Symbol.species] = C; \
             let out = a.flatMap(function(value) { return value; }); \
             (out === instance) + ':' + calls + ':' + lengthArg + ':' + out[0] + ':' + out[1] + ':' + Object.prototype.hasOwnProperty.call(out, 'length');"
        ),
        Ok(Value::String("true:1:0:1:2:false".to_owned().into()))
    );
}

#[test]
fn rejects_array_flat_map_species_result_write_failures() {
    assert_eq!(
        eval(
            "function C() { this.length = 0; Object.preventExtensions(this); } \
             let a = [[1]]; a.constructor = {}; a.constructor[Symbol.species] = C; \
             let caught = false; try { a.flatMap(function(value) { return value; }); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "function C() { Object.defineProperty(this, '0', { set: function() {}, configurable: false }); } \
             let a = [1]; a.constructor = {}; a.constructor[Symbol.species] = C; \
             let caught = false; try { a.flatMap(function(value) { return value; }); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn evaluates_array_flat_map_proxy_access_order() {
    assert_eq!(
        eval(
            "let getCalls = []; let hasCalls = []; \
             let handler = { \
               get: function(target, key) { getCalls[getCalls.length] = key; return target[key]; }, \
               has: function(target, key) { hasCalls[hasCalls.length] = key; return Reflect.has(target, key); } \
             }; \
             let tier2 = new Proxy([4, 3], handler); \
             let tier1 = new Proxy([2, [3, 4, 2, 2], 5, tier2, 6], handler); \
             Array.prototype.flatMap.call(tier1, function(value) { return value; }); \
             getCalls.join(',') + '|' + hasCalls.join(',');"
        ),
        Ok(Value::String(
            "length,constructor,0,1,2,3,length,0,1,4|0,1,2,3,0,1,4"
                .to_owned()
                .into(),
        ))
    );
}
