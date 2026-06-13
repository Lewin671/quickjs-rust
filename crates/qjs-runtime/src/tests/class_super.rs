use crate::{Value, eval};

#[test]
fn super_property_assignment_to_null_base_evaluates_rhs_before_type_error() {
    assert_eq!(
        eval(
            "var count = 0; \
             class C { static m() { super.x = count += 1; } } \
             Object.setPrototypeOf(C, null); \
             var caught = false; \
             try { C.m(); } catch (error) { caught = error instanceof TypeError; } \
             caught + ':' + count;"
        ),
        Ok(Value::String("true:1".to_owned()))
    );
}

#[test]
fn computed_super_property_assignment_to_null_base_evaluates_rhs_before_type_error() {
    assert_eq!(
        eval(
            "var count = 0; \
             class C { static m() { super[0] = count += 1; } } \
             Object.setPrototypeOf(C, null); \
             var caught = false; \
             try { C.m(); } catch (error) { caught = error instanceof TypeError; } \
             caught + ':' + count;"
        ),
        Ok(Value::String("true:1".to_owned()))
    );
}
