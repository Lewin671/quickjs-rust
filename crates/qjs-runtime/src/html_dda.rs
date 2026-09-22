use crate::{Function, NativeFunction, Value};

/// The Test262 `[[IsHTMLDDA]]` host object. The internal slot is the native
/// function's kind, so no property of any object can forge it, and the check
/// every truthiness and `typeof` test runs is a tag comparison rather than a
/// property lookup.
pub(crate) fn new_is_html_dda_function() -> Function {
    Function::new_native(Some("[[IsHTMLDDA]]"), 0, NativeFunction::IsHtmlDda, false)
}

pub(crate) fn native_is_html_dda() -> Value {
    Value::Null
}

#[inline]
pub(crate) fn is_html_dda(value: &Value) -> bool {
    matches!(value, Value::Function(function)
        if matches!(function.native_kind(), Some(NativeFunction::IsHtmlDda)))
}

#[cfg(test)]
mod tests {
    use crate::{Value, eval};

    #[test]
    fn is_html_dda_uses_annex_b_undefined_emulation() {
        assert_eq!(
            eval(
                "let v = __quickjsRustIsHTMLDDA; typeof v + ':' + !v + ':' + (v == null) + ':' + (v == undefined) + ':' + (v === undefined) + ':' + v();"
            ),
            Ok(Value::String(
                "undefined:true:true:true:false:null".to_owned().into()
            ))
        );
    }

    #[test]
    fn no_property_forges_is_html_dda() {
        assert_eq!(
            eval(
                "let o = {}; o['\\0IsHTMLDDA'] = true; function f() {} f['\\0IsHTMLDDA'] = true; \
                 [!o, typeof o, o == null, !f, typeof f, f == null, \
                  Object.getOwnPropertyNames(__quickjsRustIsHTMLDDA).length].join(':');"
            ),
            Ok(Value::String(
                "false:object:false:false:function:false:2"
                    .to_owned()
                    .into()
            ))
        );
    }
}
