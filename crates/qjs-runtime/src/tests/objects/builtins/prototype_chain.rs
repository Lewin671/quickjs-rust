use crate::{Value, eval};

#[test]
fn evaluates_object_prototype_chain_checks() {
    assert_eq!(
        eval("Object.prototype.isPrototypeOf.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval(
            "let proto = { value: 1 }; let object = Object.create(proto); proto.isPrototypeOf(object);"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let proto = { value: 1 }; let object = Object.create(proto); Object.prototype.isPrototypeOf(object);"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Object.prototype.isPrototypeOf({});"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Object.prototype.isPrototypeOf([1, 2]);"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("function F() {} Object.prototype.isPrototypeOf(F);"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("function F() {} F.prototype.isPrototypeOf(F);"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval("function F() {} Number.isPrototypeOf(new F());"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval(
            "function User(name) { this.name = name; } function ForcedUser(name, grade) { this.name = name; this.grade = grade; } let proto = new User('noname'); ForcedUser.prototype = proto; let luke = new ForcedUser('Luke Skywalker', 12); proto.isPrototypeOf(luke) && User.prototype.isPrototypeOf(luke) && !Number.isPrototypeOf(luke);"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Object.prototype.isPrototypeOf(1);"),
        Ok(Value::Boolean(false))
    );
}

#[test]
fn allows_functions_as_prototype_values() {
    // Object.create(fn): the new object inherits the function's own properties.
    assert_eq!(
        eval("function F() {} F.foo = 42; let o = Object.create(F); o.foo;"),
        Ok(Value::Number(42.0))
    );
    // getPrototypeOf returns the actual function identity.
    assert_eq!(
        eval("function F() {} let o = Object.create(F); Object.getPrototypeOf(o) === F;"),
        Ok(Value::Boolean(true))
    );
    // setPrototypeOf(obj, fn) and getPrototypeOf round-trip the function.
    assert_eq!(
        eval(
            "function F() {} let o = {}; Object.setPrototypeOf(o, F); Object.getPrototypeOf(o) === F;"
        ),
        Ok(Value::Boolean(true))
    );
    // A function inherited through setPrototypeOf still exposes its inherited
    // Function.prototype members (here `call`).
    assert_eq!(
        eval("function F() {} let o = {}; Object.setPrototypeOf(o, F); typeof o.call;"),
        Ok(Value::String("function".to_owned().into()))
    );
    // A function sitting mid-chain is walked for property reads.
    assert_eq!(
        eval(
            "function F() {} F.bar = 5; let o = Object.create(F); let p = Object.create(o); p.bar;"
        ),
        Ok(Value::Number(5.0))
    );
    // Own properties shadow the function prototype.
    assert_eq!(
        eval("function F() {} F.foo = 1; let o = Object.create(F); o.foo = 2; o.foo;"),
        Ok(Value::Number(2.0))
    );
    // isPrototypeOf finds a function mid-chain by identity.
    assert_eq!(
        eval(
            "function F() {} let o = Object.create(F); let p = Object.create(o); F.isPrototypeOf(p);"
        ),
        Ok(Value::Boolean(true))
    );
    // Reflect mirrors Object for function prototypes.
    assert_eq!(
        eval("function F() {} let o = Reflect.getPrototypeOf(Object.create(F)) === F; o;"),
        Ok(Value::Boolean(true))
    );
}
