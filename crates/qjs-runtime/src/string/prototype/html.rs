use std::collections::HashMap;

use crate::{RuntimeError, Value, to_js_string_with_env};

use super::super::indexing::this_string_value;
use crate::CallEnv;

pub(crate) enum StringHtmlKind {
    Anchor,
    Big,
    Blink,
    Bold,
    Fixed,
    Fontcolor,
    Fontsize,
    Italics,
    Link,
    Small,
    Strike,
    Sub,
    Sup,
}

pub(crate) fn native_string_prototype_html(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
    kind: StringHtmlKind,
) -> Result<Value, RuntimeError> {
    let value = this_string_value(this_value, env)?;
    Ok(Value::String(match kind {
        StringHtmlKind::Anchor => html_with_attribute(
            "a",
            "name",
            argument_values.first().cloned().unwrap_or(Value::Undefined),
            value,
            env,
        )?,
        StringHtmlKind::Big => html_tag("big", value),
        StringHtmlKind::Blink => html_tag("blink", value),
        StringHtmlKind::Bold => html_tag("b", value),
        StringHtmlKind::Fixed => html_tag("tt", value),
        StringHtmlKind::Fontcolor => html_with_attribute(
            "font",
            "color",
            argument_values.first().cloned().unwrap_or(Value::Undefined),
            value,
            env,
        )?,
        StringHtmlKind::Fontsize => html_with_attribute(
            "font",
            "size",
            argument_values.first().cloned().unwrap_or(Value::Undefined),
            value,
            env,
        )?,
        StringHtmlKind::Italics => html_tag("i", value),
        StringHtmlKind::Link => html_with_attribute(
            "a",
            "href",
            argument_values.first().cloned().unwrap_or(Value::Undefined),
            value,
            env,
        )?,
        StringHtmlKind::Small => html_tag("small", value),
        StringHtmlKind::Strike => html_tag("strike", value),
        StringHtmlKind::Sub => html_tag("sub", value),
        StringHtmlKind::Sup => html_tag("sup", value),
    }))
}

fn html_tag(tag: &str, value: String) -> String {
    format!("<{tag}>{value}</{tag}>")
}

fn html_with_attribute(
    tag: &str,
    attribute_name: &str,
    attribute_value: Value,
    value: String,
    env: &mut CallEnv,
) -> Result<String, RuntimeError> {
    let attribute_value = to_js_string_with_env(attribute_value, env)?.replace('"', "&quot;");
    Ok(format!(
        "<{tag} {attribute_name}=\"{attribute_value}\">{value}</{tag}>"
    ))
}
