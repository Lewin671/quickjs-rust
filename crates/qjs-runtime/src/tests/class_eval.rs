use crate::{Value, eval};

#[test]
fn class_field_direct_eval_inherits_initializer_context() {
    assert_eq!(
        eval(
            "var executed = false; \
             class A { get x() { return 9; } } \
             class C extends A { field = eval('executed = true; () => super.x;'); } \
             var c = new C(); c.field.call(c) + ':' + executed;"
        ),
        Ok(Value::String("9:true".to_owned()))
    );
    assert_eq!(
        eval("(function() { class C { field = eval('new.target;'); } return new C().field; })();"),
        Ok(Value::Undefined)
    );
}

#[test]
fn class_field_nested_arrow_eval_keeps_initializer_context() {
    assert_eq!(
        eval(
            "var executed = false; \
             class C { field = () => { var nested = () => eval('executed = true; arguments;'); nested(); }; } \
             var caught = false; \
             try { new C().field(); } catch (error) { caught = error instanceof SyntaxError; } \
             caught + ':' + executed;"
        ),
        Ok(Value::String("true:false".to_owned()))
    );
}

#[test]
fn class_field_eval_applies_initializer_early_errors() {
    assert_eq!(
        eval(
            "var executed = false; \
             class C { field = eval('executed = true; arguments;'); } \
             var caught = false; \
             try { new C(); } catch (error) { caught = error instanceof SyntaxError; } \
             caught + ':' + executed;"
        ),
        Ok(Value::String("true:false".to_owned()))
    );
    assert_eq!(
        eval(
            "var executed = false; \
             class A {} \
             class C extends A { field = eval('executed = true; () => super();'); } \
             var caught = false; \
             try { new C().field(); } catch (error) { caught = error instanceof SyntaxError; } \
             caught + ':' + executed;"
        ),
        Ok(Value::String("true:false".to_owned()))
    );
}

#[test]
fn class_field_indirect_eval_uses_global_script_context() {
    assert_eq!(
        eval(
            "var executed = false; \
             class C { field = (0, eval)('executed = true; new.target;'); } \
             var caught = false; \
             try { new C(); } catch (error) { caught = error instanceof SyntaxError; } \
             caught + ':' + executed;"
        ),
        Ok(Value::String("true:false".to_owned()))
    );
    assert_eq!(
        eval(
            "class A {} \
             class C extends A { field = (0, eval)('() => super.x;'); } \
             var caught = false; \
             try { new C().field(); } catch (error) { caught = error instanceof SyntaxError; } \
             caught;"
        ),
        Ok(Value::Boolean(true))
    );
}
