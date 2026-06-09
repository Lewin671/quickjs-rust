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
