use crate::{Value, eval};

#[test]
fn default_parameter_initializers_use_parameter_tdz() {
    let self_ref = eval(
        "let calls = 0; function f(x = x) { calls = calls + 1; } \
         let name; try { f(); } catch (error) { name = error.name; } \
         name + ':' + calls;",
    );
    assert_eq!(self_ref, Ok(Value::String("ReferenceError:0".to_owned())));

    let later_ref = eval(
        "let calls = 0; function f(x = y, y) { calls = calls + 1; } \
         let name; try { f(); } catch (error) { name = error.name; } \
         name + ':' + calls;",
    );
    assert_eq!(later_ref, Ok(Value::String("ReferenceError:0".to_owned())));

    assert_eq!(
        eval("function f(x = 1, y = x + 1) { return x + y; } f();"),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval("function f(x = y, y) { return x + y; } f(1, 2);"),
        Ok(Value::Number(3.0))
    );
}

#[test]
fn default_parameter_closures_do_not_capture_body_var_environment() {
    assert_eq!(
        eval(
            "var x = 'outside'; var probeParams, probeBody; \
             function f(_ = probeParams = function() { return x; }) { \
               var x = 'inside'; \
               probeBody = function() { return x; }; \
             } \
             f(); \
             probeParams() + ':' + probeBody();"
        ),
        Ok(Value::String("outside:inside".to_owned()))
    );
}
