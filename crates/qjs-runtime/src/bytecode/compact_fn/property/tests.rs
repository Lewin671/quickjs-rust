use crate::{Value, eval};

#[test]
fn named_reads_observe_descriptor_and_prototype_changes() {
    let source = r#"
        function read(o) { return o.x; }
        var o = { x: 1, y: 0 };
        var result = [read(o), read(o)];
        o.x = 2;
        result.push(read(o));
        var calls = 0;
        Object.defineProperty(o, 'x', {
            configurable: true,
            get: function () { calls++; this.y++; return 3; }
        });
        result.push(read(o), read(o), calls, o.y);
        delete o.x;
        Object.setPrototypeOf(o, { x: 4 });
        result.push(read(o), read(o));
        o.x = 5;
        result.push(read(o));
        delete o.x;
        Object.setPrototypeOf(o, { get x() { return this.y + 10; } });
        result.push(read(o));
        result.join(',');
    "#;
    assert_eq!(
        eval(source),
        Ok(Value::String("1,1,2,3,3,2,2,4,4,5,12".into()))
    );
}

#[test]
fn named_reads_fall_back_for_proxies_primitives_and_null() {
    let source = r#"
        function read(o) { return o.x; }
        var target = { x: 7 };
        var traps = 0;
        var proxy = new Proxy(target, {
            get: function (o, key, receiver) {
                traps++;
                if (key === 'x') return Reflect.get(o, key, receiver) + 1;
                return Reflect.get(o, key, receiver);
            }
        });
        var result = [read(target), read(proxy), read(proxy), traps];
        Object.defineProperty(Number.prototype, 'x', {
            configurable: true, get: function () { return this.valueOf() + 10; }
        });
        result.push(read(2));
        delete Number.prototype.x;
        try { read(null); } catch (e) { result.push(e instanceof TypeError); }
        try { read(undefined); } catch (e) { result.push(e instanceof TypeError); }
        result.join(',');
    "#;
    assert_eq!(
        eval(source),
        Ok(Value::String("7,8,8,2,12,true,true".into()))
    );
}

#[test]
fn captured_member_field_initializers_read_live_values_and_getters() {
    let source = r#"
        function exercise() {
            var source = { color: 3 };
            class Point { color = source.color; }
            var result = [new Point().color, new Point().color];
            source.color = 5;
            result.push(new Point().color);
            var calls = 0;
            Object.defineProperty(source, 'color', {
                configurable: true, get: function () { calls++; return 9; }
            });
            result.push(new Point().color, new Point().color, calls);
            source = new Proxy({ color: 12 }, {
                get: function (o, key) { calls++; return o[key]; }
            });
            result.push(new Point().color, calls);
            return result.join(',');
        }
        exercise();
    "#;
    assert_eq!(eval(source), Ok(Value::String("3,3,5,9,9,2,12,3".into())));
}
