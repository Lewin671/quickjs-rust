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

    /// `x == null` in a compiled function answers without the general
    /// operator, and still treats IsHTMLDDA as `undefined`.
    #[test]
    fn loose_null_comparison_in_a_function_keeps_is_html_dda() {
        assert_eq!(
            eval(
                "function eq(a, b) { var n = 0; if (a == b) n += 1; if (a != b) n += 10; return n; }
                 var v = __quickjsRustIsHTMLDDA, o = {}, out = [];
                 for (var i = 0; i < 3; i++) {
                     out.push(eq(v, null), eq(undefined, v), eq(o, null), eq(null, o), eq(0, null),
                              eq('', undefined), eq(null, undefined), eq(1, 1), eq(NaN, NaN), eq(0, -0));
                 }
                 out.join(':');"
            ),
            Ok(Value::String(
                "1:1:10:10:10:10:1:1:10:1:".repeat(3).trim_end_matches(':').into()
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
