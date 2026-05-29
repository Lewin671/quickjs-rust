use crate::{Value, eval};

#[test]
fn evaluates_array_flat_map_basic_mapping_and_flattening() {
    assert_eq!(
        eval("[1, 2, 3].flatMap(function(value) { return [value, value * 2]; }).join();"),
        Ok(Value::String("1,2,2,4,3,6".to_owned()))
    );
    assert_eq!(
        eval("[1, 2].flatMap(function(value) { return [[value]]; })[0][0];"),
        Ok(Value::Number(1.0))
    );
}

#[test]
fn evaluates_array_flat_map_callback_arguments_and_this_arg() {
    assert_eq!(
        eval(
            "let source = [10, 20]; let seen = ''; let out = source.flatMap(function(value, index, array) { seen = seen + value + ':' + index + ':' + (array === source) + ':' + this.offset + ';'; return [value + index + this.offset]; }, { offset: 3 }); seen + '|' + out.join();"
        ),
        Ok(Value::String("10:0:true:3;20:1:true:3;|13,24".to_owned()))
    );
}

#[test]
fn rejects_array_flat_map_non_callable_callback() {
    assert!(eval("[1].flatMap(null);").is_err());
}
