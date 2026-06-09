use crate::{Function, NativeFunction, Property, Value};

const HTML_DDA_MARKER: &str = "\0IsHTMLDDA";

pub(crate) fn new_is_html_dda_function() -> Function {
    let function = Function::new_native(Some("[[IsHTMLDDA]]"), 0, NativeFunction::IsHtmlDda, false);
    function.define_property(
        HTML_DDA_MARKER.to_owned(),
        Property::fixed_non_enumerable(Value::Boolean(true)),
    );
    function
}

pub(crate) fn native_is_html_dda() -> Value {
    Value::Null
}

pub(crate) fn is_html_dda(value: &Value) -> bool {
    match value {
        Value::Function(function) => function
            .own_property(HTML_DDA_MARKER)
            .is_some_and(|property| matches!(property.value, Value::Boolean(true))),
        Value::Object(object) => object
            .own_property(HTML_DDA_MARKER)
            .is_some_and(|property| matches!(property.value, Value::Boolean(true))),
        _ => false,
    }
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
                "undefined:true:true:true:false:null".to_owned()
            ))
        );
    }
}
