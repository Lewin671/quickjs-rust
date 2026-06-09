use crate::{Value, eval};

#[test]
fn evaluates_proxy_constructor_and_basic_traps() {
    assert_eq!(
        eval("typeof Proxy + ':' + Proxy.length;"),
        Ok(Value::String("function:2".to_owned()))
    );
    assert_eq!(
        eval(
            "let p = new Proxy({ value: 1 }, { get: function(target, key) { return key === 'value' ? 7 : target[key]; } }); p.value;"
        ),
        Ok(Value::Number(7.0))
    );
    assert_eq!(
        eval(
            "let p = new Proxy({}, { has: function(target, key) { return key === 'present'; } }); ('present' in p) + ':' + ('missing' in p);"
        ),
        Ok(Value::String("true:false".to_owned()))
    );
    assert_eq!(
        eval(
            "let deleted = ''; let p = new Proxy({ value: 1 }, { deleteProperty: function(target, key) { deleted = key; return true; } }); Reflect.deleteProperty(p, 'value'); deleted;"
        ),
        Ok(Value::String("value".to_owned()))
    );
}

#[test]
fn evaluates_proxy_revocable_and_revoked_operations() {
    assert_eq!(
        eval(
            "let r = Proxy.revocable({ value: 1 }, {}); r.proxy.value + ':' + typeof r.revoke + ':' + Proxy.revocable.length;"
        ),
        Ok(Value::String("1:function:2".to_owned()))
    );
    assert_eq!(
        eval(
            "let r = Proxy.revocable({ value: 1 }, {}); r.revoke(); r.revoke(); let caught = false; try { r.proxy.value; } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let r = Proxy.revocable([], {}); r.revoke(); let caught = false; try { [].concat(r.proxy); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn evaluates_proxy_traps_used_by_array_operations() {
    assert_eq!(
        eval(
            "let caught = false; let p = new Proxy({ length: 1 }, { has: function() { throw new TypeError('has'); } }); try { Array.prototype.copyWithin.call(p, 0, 0); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let caught = false; let p = new Proxy({ 42: true, length: 43 }, { deleteProperty: function(target, key) { if (key === '42') { throw new TypeError('delete'); } return true; } }); try { Array.prototype.copyWithin.call(p, 42, 0); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let p = new Proxy([], { get: function(target, key) { if (key === 'length') { return Number.MAX_SAFE_INTEGER; } return target[key]; } }); let caught = false; try { [].concat(1, p); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
}
