use crate::{Value, eval};

#[test]
fn evaluates_object_enumeration_builtins() {
    assert_eq!(eval("Object.keys.length;"), Ok(Value::Number(1.0)));
    assert_eq!(
        eval("Object.keys({ value: 1 })[0];"),
        Ok(Value::String("value".to_owned()))
    );
    assert_eq!(eval("Object.keys([1, 2]).length;"), Ok(Value::Number(2.0)));
    assert_eq!(
        eval("Object.keys(Object.create({ value: 1 })).length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(eval("Object.keys(Object).length;"), Ok(Value::Number(0.0)));
    assert_eq!(
        eval("Object.keys(Object.prototype).length;"),
        Ok(Value::Number(0.0))
    );
    assert!(eval("Object.keys(null);").is_err());
    assert!(eval("Object.keys(undefined);").is_err());
    assert_eq!(eval("Object.values.length;"), Ok(Value::Number(1.0)));
    assert_eq!(
        eval("Object.values({ first: 1, second: 2 }).join();"),
        Ok(Value::String("1,2".to_owned()))
    );
    assert_eq!(
        eval("Object.values([1, 2]).join();"),
        Ok(Value::String("1,2".to_owned()))
    );
    assert_eq!(
        eval(
            "Object.values(Object.create({ inherited: 1 }, { own: { value: 2, enumerable: true } }))[0];"
        ),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval("Object.values('ab').join();"),
        Ok(Value::String("a,b".to_owned()))
    );
    assert_eq!(
        eval(
            "let object = { a: 'A', get b() { this.c = 'C'; return 'B'; } }; let values = Object.values(object); values.length + ':' + values[0] + ':' + values[1] + ':' + object.c;"
        ),
        Ok(Value::String("2:A:B:C".to_owned()))
    );
    assert_eq!(
        eval(
            "let caught = false; try { Object.values({ get a() { throw new RangeError('x'); } }); } catch (error) { caught = error instanceof RangeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(eval("Object.values(0).length;"), Ok(Value::Number(0.0)));
    assert_eq!(eval("Object.entries.length;"), Ok(Value::Number(1.0)));
    assert_eq!(
        eval(
            "let entries = Object.entries({ first: 1, second: 2 }); entries[0][0] + ':' + entries[0][1] + '|' + entries[1][0] + ':' + entries[1][1];"
        ),
        Ok(Value::String("first:1|second:2".to_owned()))
    );
    assert_eq!(
        eval(
            "let entries = Object.entries([4, 5]); entries[0][0] + ':' + entries[0][1] + '|' + entries[1][0] + ':' + entries[1][1];"
        ),
        Ok(Value::String("0:4|1:5".to_owned()))
    );
    assert_eq!(
        eval(
            "let entries = Object.entries(Object.create({ inherited: 1 }, { own: { value: 2, enumerable: true } })); entries.length + ':' + entries[0][0] + ':' + entries[0][1];"
        ),
        Ok(Value::String("1:own:2".to_owned()))
    );
    assert_eq!(
        eval(
            "let entries = Object.entries('ab'); entries[0][0] + ':' + entries[0][1] + '|' + entries[1][0] + ':' + entries[1][1];"
        ),
        Ok(Value::String("0:a|1:b".to_owned()))
    );
    assert_eq!(
        eval(
            "let object = { a: 'A', get b() { return 'B'; } }; let entries = Object.entries(object); entries.length + ':' + entries[0][0] + ':' + entries[0][1] + ':' + entries[1][0] + ':' + entries[1][1];"
        ),
        Ok(Value::String("2:a:A:b:B".to_owned()))
    );
    assert_eq!(eval("Object.entries(0).length;"), Ok(Value::Number(0.0)));
    assert_eq!(
        eval("Object.getOwnPropertyNames.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("Object.getOwnPropertyNames({ value: 1 })[0];"),
        Ok(Value::String("value".to_owned()))
    );
    assert_eq!(
        eval("Object.getOwnPropertyNames([1, 2]).length;"),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval("Object.getOwnPropertyNames(Object.prototype).length;"),
        Ok(Value::Number(7.0))
    );
    assert_eq!(
        eval("Object.getOwnPropertyNames(Object.prototype)[0];"),
        Ok(Value::String("constructor".to_owned()))
    );
    assert_eq!(eval("Object.hasOwn.length;"), Ok(Value::Number(2.0)));
    assert_eq!(
        eval("Object.hasOwn({ value: 1 }, 'value');"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Object.hasOwn({ value: 1 }, 'missing');"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval(
            "let proto = { value: 1 }; let object = Object.create(proto); Object.hasOwn(object, 'value');"
        ),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval(
            "let object = Object.create(null, { value: { value: 1 } }); Object.hasOwn(object, 'value');"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Object.hasOwn([1, 2], '1');"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(eval("Object.hasOwn('ab', '1');"), Ok(Value::Boolean(true)));
}
