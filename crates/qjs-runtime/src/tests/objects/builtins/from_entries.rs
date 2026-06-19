use crate::{Value, eval};

#[test]
fn evaluates_object_from_entries() {
    assert_eq!(eval("Object.fromEntries.length;"), Ok(Value::Number(1.0)));
    assert_eq!(
        eval("let result = Object.fromEntries([['key', 'value']]); result.key;"),
        Ok(Value::String("value".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let result = Object.fromEntries([['a', 1], ['a', 2], [3, 4]]); result.a + result[3];"
        ),
        Ok(Value::Number(6.0))
    );
    assert_eq!(
        eval("let entry = { 0: 'name', 1: 'value' }; Object.fromEntries([entry]).name;"),
        Ok(Value::String("value".to_owned().into()))
    );
    assert_eq!(
        eval("Object.getPrototypeOf(Object.fromEntries([])) === Object.prototype;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let result = Object.fromEntries([['x', 1]]); let d = Object.getOwnPropertyDescriptor(result, 'x'); d.value + ':' + d.enumerable + ':' + d.writable + ':' + d.configurable;"
        ),
        Ok(Value::String("1:true:true:true".to_owned().into()))
    );
    assert_eq!(
        eval("let key = Symbol(); let result = Object.fromEntries([[key, 'value']]); result[key];"),
        Ok(Value::String("value".to_owned().into()))
    );
    assert_eq!(
        eval("let result = Object.fromEntries([[['nested'], 'value']].values()); result.nested;"),
        Ok(Value::String("value".to_owned().into()))
    );
    assert!(eval("Object.fromEntries();").is_err());
    assert!(eval("Object.fromEntries(['ab']);").is_err());
}

#[test]
fn iterator_closed_for_null_entry() {
    // When an entry is null, the iterator must be closed and a TypeError thrown.
    assert_eq!(
        eval(
            r#"
            var returned = false;
            var iterable = {
                [Symbol.iterator]: function() {
                    var advanced = false;
                    return {
                        next: function() {
                            if (advanced) throw new Error('should only advance once');
                            advanced = true;
                            return { done: false, value: null };
                        },
                        return: function() {
                            if (returned) throw new Error('should only return once');
                            returned = true;
                        },
                    };
                },
            };
            var threw = false;
            try {
                Object.fromEntries(iterable);
            } catch (e) {
                threw = e instanceof TypeError;
            }
            threw + ':' + returned;
            "#
        ),
        Ok(Value::String("true:true".to_owned().into()))
    );
}

#[test]
fn iterator_closed_for_string_entry() {
    // When an entry is a string (primitive, not object), the iterator must be
    // closed and a TypeError thrown.
    assert_eq!(
        eval(
            r#"
            var returned = false;
            var iterable = {
                [Symbol.iterator]: function() {
                    var advanced = false;
                    return {
                        next: function() {
                            if (advanced) throw new Error('should only advance once');
                            advanced = true;
                            return { done: false, value: 'ab' };
                        },
                        return: function() {
                            if (returned) throw new Error('should only return once');
                            returned = true;
                        },
                    };
                },
            };
            var threw = false;
            try {
                Object.fromEntries(iterable);
            } catch (e) {
                threw = e instanceof TypeError;
            }
            threw + ':' + returned;
            "#
        ),
        Ok(Value::String("true:true".to_owned().into()))
    );
}

#[test]
fn iterator_closed_for_number_entry() {
    // When an entry is a number, the iterator must be closed.
    assert_eq!(
        eval(
            r#"
            var returned = false;
            var iterable = {
                [Symbol.iterator]: function() {
                    var advanced = false;
                    return {
                        next: function() {
                            if (advanced) throw new Error('should only advance once');
                            advanced = true;
                            return { done: false, value: 42 };
                        },
                        return: function() {
                            returned = true;
                        },
                    };
                },
            };
            var threw = false;
            try {
                Object.fromEntries(iterable);
            } catch (e) {
                threw = e instanceof TypeError;
            }
            threw + ':' + returned;
            "#
        ),
        Ok(Value::String("true:true".to_owned().into()))
    );
}

#[test]
fn iterator_closed_for_undefined_entry() {
    // When an entry is undefined, the iterator must be closed.
    assert_eq!(
        eval(
            r#"
            var returned = false;
            var iterable = {
                [Symbol.iterator]: function() {
                    var advanced = false;
                    return {
                        next: function() {
                            if (advanced) throw new Error('should only advance once');
                            advanced = true;
                            return { done: false, value: undefined };
                        },
                        return: function() {
                            returned = true;
                        },
                    };
                },
            };
            var threw = false;
            try {
                Object.fromEntries(iterable);
            } catch (e) {
                threw = e instanceof TypeError;
            }
            threw + ':' + returned;
            "#
        ),
        Ok(Value::String("true:true".to_owned().into()))
    );
}

#[test]
fn iterator_closed_for_boolean_entry() {
    // When an entry is a boolean, the iterator must be closed.
    assert_eq!(
        eval(
            r#"
            var returned = false;
            var iterable = {
                [Symbol.iterator]: function() {
                    var advanced = false;
                    return {
                        next: function() {
                            if (advanced) throw new Error('should only advance once');
                            advanced = true;
                            return { done: false, value: true };
                        },
                        return: function() {
                            returned = true;
                        },
                    };
                },
            };
            var threw = false;
            try {
                Object.fromEntries(iterable);
            } catch (e) {
                threw = e instanceof TypeError;
            }
            threw + ':' + returned;
            "#
        ),
        Ok(Value::String("true:true".to_owned().into()))
    );
}

#[test]
fn iterator_closed_for_throwing_key_accessor() {
    // When accessing property "0" (key) of the entry throws, the iterator
    // must be closed and the original error propagated.
    assert_eq!(
        eval(
            r#"
            function DummyError() {}
            var returned = false;
            var iterable = {
                [Symbol.iterator]: function() {
                    var advanced = false;
                    return {
                        next: function() {
                            if (advanced) throw new Error('should only advance once');
                            advanced = true;
                            return {
                                done: false,
                                value: {
                                    get '0'() { throw new DummyError(); },
                                },
                            };
                        },
                        return: function() {
                            if (returned) throw new Error('should only return once');
                            returned = true;
                        },
                    };
                },
            };
            var threwDummy = false;
            try {
                Object.fromEntries(iterable);
            } catch (e) {
                threwDummy = e instanceof DummyError;
            }
            threwDummy + ':' + returned;
            "#
        ),
        Ok(Value::String("true:true".to_owned().into()))
    );
}

#[test]
fn iterator_closed_for_throwing_value_accessor() {
    // When accessing property "1" (value) of the entry throws, the iterator
    // must be closed and the original error propagated.
    assert_eq!(
        eval(
            r#"
            function DummyError() {}
            var returned = false;
            var iterable = {
                [Symbol.iterator]: function() {
                    var advanced = false;
                    return {
                        next: function() {
                            if (advanced) throw new Error('should only advance once');
                            advanced = true;
                            return {
                                done: false,
                                value: {
                                    get '0'() { return 'key'; },
                                    get '1'() { throw new DummyError(); },
                                },
                            };
                        },
                        return: function() {
                            if (returned) throw new Error('should only return once');
                            returned = true;
                        },
                    };
                },
            };
            var threwDummy = false;
            try {
                Object.fromEntries(iterable);
            } catch (e) {
                threwDummy = e instanceof DummyError;
            }
            threwDummy + ':' + returned;
            "#
        ),
        Ok(Value::String("true:true".to_owned().into()))
    );
}

#[test]
fn iterator_closed_for_throwing_key_tostring() {
    // When toString on a key throws, the iterator must be closed and the
    // original error propagated.
    assert_eq!(
        eval(
            r#"
            function DummyError() {}
            var returned = false;
            var iterable = {
                [Symbol.iterator]: function() {
                    var advanced = false;
                    return {
                        next: function() {
                            if (advanced) throw new Error('should only advance once');
                            advanced = true;
                            return {
                                done: false,
                                value: {
                                    0: { toString: function() { throw new DummyError(); } },
                                },
                            };
                        },
                        return: function() {
                            if (returned) throw new Error('should only return once');
                            returned = true;
                        },
                    };
                },
            };
            var threwDummy = false;
            try {
                Object.fromEntries(iterable);
            } catch (e) {
                threwDummy = e instanceof DummyError;
            }
            threwDummy + ':' + returned;
            "#
        ),
        Ok(Value::String("true:true".to_owned().into()))
    );
}

#[test]
fn evaluation_order() {
    // Evaluation order must be:
    // next() -> get "0" -> get "1" -> toPropertyKey -> next() -> ...
    assert_eq!(
        eval(
            r#"
            var effects = [];
            function makeEntry(label) {
                return {
                    get '0'() {
                        effects.push('access "0" of ' + label);
                        return {
                            toString: function() {
                                effects.push('toString of ' + label + ' key');
                                return label + ' key';
                            },
                        };
                    },
                    get '1'() {
                        effects.push('access "1" of ' + label);
                        return label + ' value';
                    },
                };
            }
            var iterable = {
                [Symbol.iterator]: function() {
                    effects.push('get Symbol.iterator');
                    var count = 0;
                    return {
                        next: function() {
                            effects.push('next ' + count);
                            if (count === 0) { ++count; return { done: false, value: makeEntry('first') }; }
                            else if (count === 1) { ++count; return { done: false, value: makeEntry('second') }; }
                            else { return { done: true }; }
                        },
                    };
                },
            };
            var result = Object.fromEntries(iterable);
            var expected = [
                'get Symbol.iterator',
                'next 0',
                'access "0" of first',
                'access "1" of first',
                'toString of first key',
                'next 1',
                'access "0" of second',
                'access "1" of second',
                'toString of second key',
                'next 2',
            ];
            var orderOk = effects.length === expected.length && effects.every(function(e, i) { return e === expected[i]; });
            orderOk + ':' + result['first key'] + ':' + result['second key'];
            "#
        ),
        Ok(Value::String(
            "true:first value:second value".to_owned().into()
        ))
    );
}

#[test]
fn iterator_not_closed_when_no_return_method() {
    // If the iterator has no `return` method, fromEntries should still
    // throw on invalid entries without trying to call return.
    assert_eq!(
        eval(
            r#"
            var iterable = {
                [Symbol.iterator]: function() {
                    var advanced = false;
                    return {
                        next: function() {
                            if (advanced) throw new Error('should only advance once');
                            advanced = true;
                            return { done: false, value: null };
                        },
                    };
                },
            };
            var threw = false;
            try {
                Object.fromEntries(iterable);
            } catch (e) {
                threw = e instanceof TypeError;
            }
            String(threw);
            "#
        ),
        Ok(Value::String("true".to_owned().into()))
    );
}
