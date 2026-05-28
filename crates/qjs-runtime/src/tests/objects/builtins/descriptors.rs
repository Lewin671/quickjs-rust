use crate::{Value, eval};

#[test]
fn evaluates_object_descriptor_queries() {
    assert_eq!(
        eval("Object.getOwnPropertyDescriptor.length;"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval("let object = { value: 1 }; Object.getOwnPropertyDescriptor(object, 'value').value;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("Object.getOwnPropertyDescriptor({ value: 1 }, 'value').enumerable;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Object.getOwnPropertyDescriptor(Object.prototype, 'toString').enumerable;"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval("Object.getOwnPropertyDescriptor([1, 2], 'length').value;"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval("Object.getOwnPropertyDescriptor({}, 'missing');"),
        Ok(Value::Undefined)
    );
    assert_eq!(
        eval("Object.getOwnPropertyDescriptors.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("Object.getPrototypeOf(Object.getOwnPropertyDescriptors({})) === Object.prototype;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let descriptors = Object.getOwnPropertyDescriptors({ value: 1 }); descriptors.value.value;"
        ),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("Object.getOwnPropertyDescriptors({ value: 1 }).value.enumerable;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let object = {}; Object.defineProperty(object, 'hidden', { value: 2 }); Object.getOwnPropertyDescriptors(object).hidden.enumerable;"
        ),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval(
            "let object = Object.create({ inherited: 1 }, { own: { value: 2, enumerable: true } }); Object.keys(Object.getOwnPropertyDescriptors(object)).join();"
        ),
        Ok(Value::String("own".to_owned()))
    );
    assert_eq!(
        eval(
            "let descriptors = Object.getOwnPropertyDescriptors('ab'); descriptors.length.value + ':' + descriptors[0].value + ':' + descriptors[0].writable + ':' + descriptors[0].configurable;"
        ),
        Ok(Value::String("2:a:false:false".to_owned()))
    );
    assert_eq!(
        eval("Object.keys(Object.getOwnPropertyDescriptors(0)).length;"),
        Ok(Value::Number(0.0))
    );
    assert!(eval("Object.getOwnPropertyDescriptors(null);").is_err());
    assert!(eval("Object.getOwnPropertyDescriptors(undefined);").is_err());
}
