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
