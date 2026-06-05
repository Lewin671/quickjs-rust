use crate::{Value, eval};

#[test]
fn evaluates_object_group_by_arrays() {
    assert_eq!(eval("Object.groupBy.length;"), Ok(Value::Number(2.0)));
    assert_eq!(
        eval(
            "let groups = Object.groupBy([1, 2, 3, 4], function(value) { return value % 2 ? 'odd' : 'even'; }); groups.odd.join('|') + ':' + groups.even.join('|');"
        ),
        Ok(Value::String("1|3:2|4".to_owned()))
    );
    assert_eq!(
        eval(
            "let seen = ''; let groups = Object.groupBy({ 0: 'a', 1: 'b', length: 2 }, function(value, index) { seen = seen + value + index; return index; }); Object.keys(groups).join('|') + ':' + groups[0][0] + ':' + groups[1][0] + ':' + seen;"
        ),
        Ok(Value::String("0|1:a:b:a0b1".to_owned()))
    );
    assert_eq!(
        eval("Object.getPrototypeOf(Object.groupBy([], function() { return 'x'; })) === null;"),
        Ok(Value::Boolean(true))
    );
    assert!(eval("Object.groupBy([1], 1);").is_err());
    assert!(eval("Object.groupBy(null, function(value) { return value; });").is_err());
}

#[test]
fn evaluates_object_group_by_property_keys() {
    assert_eq!(
        eval(
            "let groups = Object.groupBy([1, 2], function(value) { return value === 1; }); groups.true[0] + ':' + groups.false[0];"
        ),
        Ok(Value::String("1:2".to_owned()))
    );
    assert_eq!(
        eval(
            "let symbol = Symbol(); let groups = Object.groupBy(['x'], function() { return symbol; }); Object.getOwnPropertySymbols(groups)[0] === symbol;"
        ),
        Ok(Value::Boolean(true))
    );
}
