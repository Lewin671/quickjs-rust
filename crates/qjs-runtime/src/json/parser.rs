use std::{collections::HashMap, rc::Rc};

use crate::CallEnv;
use crate::value::OrderedDataPropertyBuilder;
use crate::{
    ArrayRef, ObjectRef, Property, PropertyKey, RuntimeError, Value, object_prototype,
    to_js_string_with_env, to_length_with_env,
};

pub(crate) fn native_json_parse(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let source = to_js_string_with_env(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    let reviver = argument_values.get(1).cloned().unwrap_or(Value::Undefined);
    if is_callable(&reviver) {
        let node = parse_json_node(&source, env)?;
        let value = node.value.clone();
        let root = Value::Object(crate::ObjectRef::with_prototype(
            std::collections::HashMap::new(),
            object_prototype(env),
        ));
        if let Value::Object(ref obj) = root {
            obj.define_property(String::new(), crate::Property::enumerable(value));
        }
        internalize_json_property(&root, "", &node, &reviver, env)
    } else {
        parse_json_text(&source, env)
    }
}

#[derive(Clone)]
struct JsonNode {
    value: Value,
    source: Option<String>,
    children: JsonNodeChildren,
}

#[derive(Clone)]
enum JsonNodeChildren {
    None,
    Array(Vec<JsonNode>),
    Object(Vec<(Rc<str>, JsonNode)>),
}

fn internalize_json_property(
    holder: &Value,
    name: &str,
    node: &JsonNode,
    reviver: &Value,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let value = crate::property_value(holder.clone(), name, env)?;
    if is_object_like(&value) {
        if is_array_object(&value)? {
            let len =
                to_length_with_env(crate::property_value(value.clone(), "length", env)?, env)?;
            for i in 0..len {
                let key = i.to_string();
                let child = array_child_node(node, i, &value, &key, env)?;
                let new_element = internalize_json_property(&value, &key, &child, reviver, env)?;
                if matches!(new_element, Value::Undefined) {
                    delete_property(value.clone(), PropertyKey::String(key), env)?;
                } else {
                    create_data_property(
                        value.clone(),
                        PropertyKey::String(key),
                        new_element,
                        env,
                    )?;
                }
            }
        } else {
            let keys = enumerable_own_string_keys(value.clone(), env)?;
            for key in keys {
                let child = object_child_node(node, &key, &value, env)?;
                let new_element = internalize_json_property(&value, &key, &child, reviver, env)?;
                if matches!(new_element, Value::Undefined) {
                    delete_property(value.clone(), PropertyKey::String(key), env)?;
                } else {
                    create_data_property(
                        value.clone(),
                        PropertyKey::String(key),
                        new_element,
                        env,
                    )?;
                }
            }
        }
    }
    crate::call_function(
        reviver.clone(),
        holder.clone(),
        vec![
            Value::String(name.to_owned().into()),
            value,
            reviver_context(node, env),
        ],
        env,
        false,
    )
}

fn is_callable(value: &Value) -> bool {
    match value {
        Value::Function(_) => true,
        Value::Proxy(proxy) => crate::proxy::proxy_is_callable(proxy),
        _ => false,
    }
}

fn is_object_like(value: &Value) -> bool {
    matches!(
        value,
        Value::Array(_)
            | Value::Object(_)
            | Value::Function(_)
            | Value::Map(_)
            | Value::Set(_)
            | Value::Proxy(_)
    )
}

fn is_array_object(value: &Value) -> Result<bool, RuntimeError> {
    match value {
        Value::Array(_) => Ok(true),
        Value::Proxy(proxy) => crate::proxy::proxy_target_is_array_result(proxy),
        _ => Ok(false),
    }
}

fn array_child_node(
    node: &JsonNode,
    index: usize,
    holder: &Value,
    key: &str,
    env: &mut CallEnv,
) -> Result<JsonNode, RuntimeError> {
    if let JsonNodeChildren::Array(children) = &node.children
        && let Some(child) = children.get(index)
    {
        return child_for_current_value(child, holder, key, env);
    }
    Ok(empty_node(crate::property_value(holder.clone(), key, env)?))
}

fn object_child_node(
    node: &JsonNode,
    key: &str,
    holder: &Value,
    env: &mut CallEnv,
) -> Result<JsonNode, RuntimeError> {
    if let JsonNodeChildren::Object(children) = &node.children {
        if let Some((_, child)) = children
            .iter()
            .rev()
            .find(|(child_key, _)| child_key.as_ref() == key)
        {
            return child_for_current_value(child, holder, key, env);
        }
    }
    Ok(empty_node(crate::property_value(holder.clone(), key, env)?))
}

fn child_for_current_value(
    child: &JsonNode,
    holder: &Value,
    key: &str,
    env: &mut CallEnv,
) -> Result<JsonNode, RuntimeError> {
    let current = crate::property_value(holder.clone(), key, env)?;
    if current.same_value(&child.value) {
        Ok(child.clone())
    } else {
        Ok(empty_node(current))
    }
}

fn empty_node(value: Value) -> JsonNode {
    JsonNode {
        value,
        source: None,
        children: JsonNodeChildren::None,
    }
}

fn reviver_context(node: &JsonNode, env: &CallEnv) -> Value {
    let context = ObjectRef::with_prototype(HashMap::new(), object_prototype(env));
    if let Some(source) = &node.source {
        context.define_property(
            "source".to_owned(),
            Property::enumerable(Value::String(source.clone().into())),
        );
    }
    Value::Object(context)
}

fn enumerable_own_string_keys(
    value: Value,
    env: &mut CallEnv,
) -> Result<Vec<String>, RuntimeError> {
    let keys = match value.clone() {
        Value::Proxy(proxy) => crate::proxy::proxy_own_keys(proxy, env)?,
        _ => crate::object::own_property_names(value.clone())
            .into_iter()
            .map(PropertyKey::String)
            .collect(),
    };
    let mut enumerable = Vec::new();
    for key in keys {
        let PropertyKey::String(name) = key else {
            continue;
        };
        let descriptor =
            own_property_descriptor(value.clone(), &PropertyKey::String(name.clone()), env)?;
        if descriptor.is_some_and(|property| property.enumerable) {
            enumerable.push(name);
        }
    }
    Ok(enumerable)
}

fn own_property_descriptor(
    value: Value,
    key: &PropertyKey,
    env: &mut CallEnv,
) -> Result<Option<Property>, RuntimeError> {
    match value {
        Value::Proxy(proxy) => {
            let forward_key = key.clone();
            crate::proxy::proxy_get_own_property_descriptor(proxy, key, env, move |target, env| {
                crate::object::own_property_descriptor_key(target, &forward_key, env)
            })
        }
        value => crate::object::own_property_descriptor_key(value, key, env),
    }
}

fn create_data_property(
    target: Value,
    key: PropertyKey,
    value: Value,
    env: &mut CallEnv,
) -> Result<(), RuntimeError> {
    crate::object::define_property_on_value_key(target, key, Property::enumerable(value), env)?;
    Ok(())
}

fn delete_property(target: Value, key: PropertyKey, env: &mut CallEnv) -> Result<(), RuntimeError> {
    match target {
        Value::Object(object) => match key {
            PropertyKey::String(key) => object.delete_own_property(&key),
            PropertyKey::Symbol(symbol) => object.delete_own_symbol_property(&symbol),
        },
        Value::Array(elements) => match key {
            PropertyKey::String(key) => match key.parse::<usize>() {
                Ok(index) => elements.delete_index(index),
                Err(_) => elements.delete_property(&key),
            },
            PropertyKey::Symbol(symbol) => elements.delete_own_symbol_property(&symbol),
        },
        Value::Function(function) => match key {
            PropertyKey::String(key) => crate::function_delete_own_property(&function, &key),
            PropertyKey::Symbol(symbol) => {
                crate::function_delete_own_symbol_property(&function, &symbol)
            }
        },
        Value::Map(map) => {
            delete_property(Value::Object(map.object()), key, env)?;
            true
        }
        Value::Set(set) => {
            delete_property(Value::Object(set.object()), key, env)?;
            true
        }
        Value::Proxy(proxy) => crate::proxy::proxy_delete_property(proxy, &key, env)?,
        Value::String(_)
        | Value::Number(_)
        | Value::BigInt(_)
        | Value::Boolean(_)
        | Value::Null
        | Value::Undefined => true,
    };
    Ok(())
}

pub(crate) fn parse_json_text(source: &str, env: &CallEnv) -> Result<Value, RuntimeError> {
    JsonParser::new(source, env, JsonSourceMode::RuntimeWtf16)
        .parse::<false>()
        .map(|node| node.value)
}

pub(crate) fn parse_host_json_text(source: &str, env: &CallEnv) -> Result<Value, RuntimeError> {
    JsonParser::new(source, env, JsonSourceMode::HostUtf8)
        .parse::<false>()
        .map(|node| node.value)
}

fn parse_json_node(source: &str, env: &CallEnv) -> Result<JsonNode, RuntimeError> {
    JsonParser::new(source, env, JsonSourceMode::RuntimeWtf16).parse::<true>()
}

#[derive(Clone, Copy)]
enum JsonSourceMode {
    RuntimeWtf16,
    HostUtf8,
}

#[derive(Debug, PartialEq, Eq)]
enum JsonStringScan {
    Direct {
        start: usize,
        end: usize,
    },
    /// Decode from `resume`, seeding the output with the source bytes between
    /// `content_start` and `resume`. Confirmed Host sentinels set both offsets
    /// to `content_start` so normalization covers the complete string.
    Decode {
        content_start: usize,
        resume: usize,
        potential_host_sentinel: bool,
    },
}

struct JsonParser<'a> {
    source: &'a str,
    cursor: usize,
    env: &'a CallEnv,
    source_mode: JsonSourceMode,
}

impl<'a> JsonParser<'a> {
    fn new(source: &'a str, env: &'a CallEnv, source_mode: JsonSourceMode) -> Self {
        Self {
            source,
            cursor: 0,
            env,
            source_mode,
        }
    }

    fn parse<const PRESERVE_METADATA: bool>(mut self) -> Result<JsonNode, RuntimeError> {
        self.skip_whitespace();
        let value = self.value::<PRESERVE_METADATA>()?;
        self.skip_whitespace();
        if self.cursor == self.source.len() {
            Ok(value)
        } else {
            Err(self.syntax_error())
        }
    }

    fn value<const PRESERVE_METADATA: bool>(&mut self) -> Result<JsonNode, RuntimeError> {
        self.skip_whitespace();
        match self.peek() {
            Some('"') => self.string_node::<PRESERVE_METADATA>(),
            Some('[') => self.array::<PRESERVE_METADATA>(),
            Some('{') => self.object::<PRESERVE_METADATA>(),
            Some('t') => self.literal::<PRESERVE_METADATA>("true", Value::Boolean(true)),
            Some('f') => self.literal::<PRESERVE_METADATA>("false", Value::Boolean(false)),
            Some('n') => self.literal::<PRESERVE_METADATA>("null", Value::Null),
            Some('-' | '0'..='9') => self.number::<PRESERVE_METADATA>(),
            _ => Err(self.syntax_error()),
        }
    }

    fn array<const PRESERVE_METADATA: bool>(&mut self) -> Result<JsonNode, RuntimeError> {
        self.expect_char('[')?;
        let mut values = Vec::new();
        let mut children = PRESERVE_METADATA.then(Vec::new);
        self.skip_whitespace();
        if self.consume_char(']') {
            return Ok(JsonNode {
                value: Value::Array(ArrayRef::new(Vec::new())),
                source: None,
                children: children
                    .map(JsonNodeChildren::Array)
                    .unwrap_or(JsonNodeChildren::None),
            });
        }

        loop {
            let child = self.value::<PRESERVE_METADATA>()?;
            if let Some(children) = children.as_mut() {
                values.push(child.value.clone());
                children.push(child);
            } else {
                values.push(child.value);
            }
            self.skip_whitespace();
            if self.consume_char(']') {
                break;
            }
            self.expect_char(',')?;
        }
        Ok(JsonNode {
            value: Value::Array(ArrayRef::new(values)),
            source: None,
            children: children
                .map(JsonNodeChildren::Array)
                .unwrap_or(JsonNodeChildren::None),
        })
    }

    fn object<const PRESERVE_METADATA: bool>(&mut self) -> Result<JsonNode, RuntimeError> {
        self.expect_char('{')?;
        let mut properties = OrderedDataPropertyBuilder::new();
        let mut children = PRESERVE_METADATA.then(Vec::new);
        self.skip_whitespace();
        if self.consume_char('}') {
            return Ok(JsonNode {
                value: Value::Object(properties.finish(object_prototype(self.env))),
                source: None,
                children: children
                    .map(JsonNodeChildren::Object)
                    .unwrap_or(JsonNodeChildren::None),
            });
        }

        loop {
            self.skip_whitespace();
            if self.peek() != Some('"') {
                return Err(self.syntax_error());
            }
            let key = self.string_rc()?;
            self.skip_whitespace();
            self.expect_char(':')?;
            let child = self.value::<PRESERVE_METADATA>()?;
            if let Some(children) = children.as_mut() {
                properties.insert(key.clone(), child.value.clone());
                children.push((key, child));
            } else {
                properties.insert(key, child.value);
            }
            self.skip_whitespace();
            if self.consume_char('}') {
                break;
            }
            self.expect_char(',')?;
        }
        Ok(JsonNode {
            value: Value::Object(properties.finish(object_prototype(self.env))),
            source: None,
            children: children
                .map(JsonNodeChildren::Object)
                .unwrap_or(JsonNodeChildren::None),
        })
    }

    fn string_node<const PRESERVE_METADATA: bool>(&mut self) -> Result<JsonNode, RuntimeError> {
        let start = self.cursor;
        let value = self.string_value_rc()?;
        Ok(JsonNode {
            value: Value::String(value),
            source: PRESERVE_METADATA.then(|| self.source[start..self.cursor].to_owned()),
            children: JsonNodeChildren::None,
        })
    }

    fn string_rc(&mut self) -> Result<Rc<str>, RuntimeError> {
        match self.scan_string()? {
            JsonStringScan::Direct { start, end } => {
                let value = Rc::from(&self.source[start..end]);
                self.cursor = end + 1;
                Ok(value)
            }
            JsonStringScan::Decode {
                content_start,
                resume,
                potential_host_sentinel,
            } => self
                .decode_string(content_start, resume, potential_host_sentinel)
                .map(Rc::from),
        }
    }

    fn string_value_rc(&mut self) -> Result<Rc<String>, RuntimeError> {
        match self.scan_string()? {
            JsonStringScan::Direct { start, end } => {
                let value = Rc::new(self.source[start..end].to_owned());
                self.cursor = end + 1;
                Ok(value)
            }
            JsonStringScan::Decode {
                content_start,
                resume,
                potential_host_sentinel,
            } => self
                .decode_string(content_start, resume, potential_host_sentinel)
                .map(Rc::new),
        }
    }

    /// Scans to the first byte that requires decoding without changing the
    /// cursor. UTF-8 continuation bytes cannot alias JSON's ASCII quote,
    /// backslash, or control bytes, so a byte scan is sufficient except for
    /// the runtime's narrow Host UTF-8 sentinel-overlap range.
    fn scan_string(&mut self) -> Result<JsonStringScan, RuntimeError> {
        self.expect_char('"')?;
        let bytes = self.source.as_bytes();
        let content_start = self.cursor;
        let check_host_sentinel = matches!(self.source_mode, JsonSourceMode::HostUtf8);
        let mut possible_host_sentinel = false;
        for (index, byte) in bytes.iter().copied().enumerate().skip(content_start) {
            if check_host_sentinel && byte == 0xf3 && bytes.get(index + 1).copied() == Some(0xb0) {
                possible_host_sentinel = true;
            }

            match byte {
                b'"' => {
                    if possible_host_sentinel {
                        let contains_sentinel = self.source[content_start..index]
                            .chars()
                            .any(|ch| crate::string::surrogate_escape_code_unit(ch).is_some());
                        if contains_sentinel {
                            return Ok(JsonStringScan::Decode {
                                content_start,
                                resume: content_start,
                                potential_host_sentinel: true,
                            });
                        }
                        return Ok(JsonStringScan::Direct {
                            start: content_start,
                            end: index,
                        });
                    }
                    return Ok(JsonStringScan::Direct {
                        start: content_start,
                        end: index,
                    });
                }
                b'\\' => {
                    let contains_sentinel = possible_host_sentinel
                        && self.source[content_start..index]
                            .chars()
                            .any(|ch| crate::string::surrogate_escape_code_unit(ch).is_some());
                    return Ok(JsonStringScan::Decode {
                        content_start,
                        resume: if contains_sentinel {
                            content_start
                        } else {
                            index
                        },
                        potential_host_sentinel: contains_sentinel,
                    });
                }
                0x00..=0x1f => {
                    return Ok(JsonStringScan::Decode {
                        content_start,
                        resume: index,
                        potential_host_sentinel: false,
                    });
                }
                _ => {}
            }
        }
        Ok(JsonStringScan::Decode {
            content_start,
            resume: bytes.len(),
            potential_host_sentinel: false,
        })
    }

    fn decode_string(
        &mut self,
        content_start: usize,
        resume: usize,
        potential_host_sentinel: bool,
    ) -> Result<String, RuntimeError> {
        debug_assert!(!potential_host_sentinel || resume == content_start);
        let output = self.source[content_start..resume].to_owned();
        self.cursor = resume;
        self.string_tail(output)
    }

    fn string_tail(&mut self, mut output: String) -> Result<String, RuntimeError> {
        loop {
            let Some(ch) = self.next_char() else {
                return Err(self.syntax_error());
            };
            match ch {
                '"' => return Ok(output),
                '\\' => crate::string::push_code_unit(&mut output, self.escape()?),
                ch if ch <= '\u{1f}' => return Err(self.syntax_error()),
                ch => match self.source_mode {
                    JsonSourceMode::RuntimeWtf16 => output.push(ch),
                    JsonSourceMode::HostUtf8 => {
                        crate::string::push_code_point(&mut output, ch as u32);
                    }
                },
            }
        }
    }

    fn escape(&mut self) -> Result<u16, RuntimeError> {
        let Some(ch) = self.next_char() else {
            return Err(self.syntax_error());
        };
        match ch {
            '"' | '\\' | '/' => Ok(ch as u16),
            'b' => Ok(0x08),
            'f' => Ok(0x0c),
            'n' => Ok(u16::from(b'\n')),
            'r' => Ok(u16::from(b'\r')),
            't' => Ok(u16::from(b'\t')),
            'u' => self.unicode_escape(),
            _ => Err(self.syntax_error()),
        }
    }

    fn unicode_escape(&mut self) -> Result<u16, RuntimeError> {
        let mut value = 0u16;
        for _ in 0..4 {
            let Some(ch) = self.next_char() else {
                return Err(self.syntax_error());
            };
            let Some(digit) = ch.to_digit(16) else {
                return Err(self.syntax_error());
            };
            value = value * 16 + digit as u16;
        }
        Ok(value)
    }

    fn number<const PRESERVE_METADATA: bool>(&mut self) -> Result<JsonNode, RuntimeError> {
        let start = self.cursor;
        self.consume_char('-');
        match self.peek() {
            Some('0') => {
                self.next_char();
                if matches!(self.peek(), Some('0'..='9')) {
                    return Err(self.syntax_error());
                }
            }
            Some('1'..='9') => {
                self.next_char();
                while matches!(self.peek(), Some('0'..='9')) {
                    self.next_char();
                }
            }
            _ => return Err(self.syntax_error()),
        }

        if self.consume_char('.') {
            if !matches!(self.peek(), Some('0'..='9')) {
                return Err(self.syntax_error());
            }
            while matches!(self.peek(), Some('0'..='9')) {
                self.next_char();
            }
        }

        if matches!(self.peek(), Some('e' | 'E')) {
            self.next_char();
            if matches!(self.peek(), Some('+' | '-')) {
                self.next_char();
            }
            if !matches!(self.peek(), Some('0'..='9')) {
                return Err(self.syntax_error());
            }
            while matches!(self.peek(), Some('0'..='9')) {
                self.next_char();
            }
        }

        let source = &self.source[start..self.cursor];
        let value = source.parse::<f64>().map_err(|_| self.syntax_error())?;
        Ok(JsonNode {
            value: Value::Number(value),
            source: PRESERVE_METADATA.then(|| source.to_owned()),
            children: JsonNodeChildren::None,
        })
    }

    fn literal<const PRESERVE_METADATA: bool>(
        &mut self,
        literal: &str,
        value: Value,
    ) -> Result<JsonNode, RuntimeError> {
        if self.source[self.cursor..].starts_with(literal) {
            self.cursor += literal.len();
            Ok(JsonNode {
                value,
                source: PRESERVE_METADATA.then(|| literal.to_owned()),
                children: JsonNodeChildren::None,
            })
        } else {
            Err(self.syntax_error())
        }
    }

    fn expect_char(&mut self, expected: char) -> Result<(), RuntimeError> {
        if self.consume_char(expected) {
            Ok(())
        } else {
            Err(self.syntax_error())
        }
    }

    fn consume_char(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.next_char();
            true
        } else {
            false
        }
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some('\t' | '\n' | '\r' | ' ')) {
            self.next_char();
        }
    }

    fn peek(&self) -> Option<char> {
        self.source[self.cursor..].chars().next()
    }

    fn next_char(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.cursor += ch.len_utf8();
        Some(ch)
    }

    fn syntax_error(&self) -> RuntimeError {
        RuntimeError {
            thrown: None,
            message: "SyntaxError: JSON syntax error".to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::function::new_realm;

    fn test_env() -> CallEnv {
        CallEnv::new(new_realm(HashMap::new()))
    }

    fn scan(source: &str, mode: JsonSourceMode) -> JsonStringScan {
        let env = test_env();
        JsonParser::new(source, &env, mode)
            .scan_string()
            .expect("test source starts with a quote")
    }

    #[test]
    fn unescaped_string_fast_path_accepts_direct_runtime_content() {
        let ascii = r#""plain ASCII""#;
        assert_eq!(
            scan(ascii, JsonSourceMode::HostUtf8),
            JsonStringScan::Direct {
                start: 1,
                end: ascii.len() - 1
            }
        );
        let unicode = r#""文😀""#;
        assert_eq!(
            scan(unicode, JsonSourceMode::RuntimeWtf16),
            JsonStringScan::Direct {
                start: 1,
                end: unicode.len() - 1
            }
        );

        let sentinel = crate::string::char_from_code_unit(0xD800);
        let source = format!("\"{sentinel}\"");
        assert_eq!(
            scan(&source, JsonSourceMode::RuntimeWtf16),
            JsonStringScan::Direct {
                start: 1,
                end: source.len() - 1
            }
        );

        // U+F0800 shares the two-byte candidate prefix but is outside the
        // runtime sentinel range, so the rare character check returns Direct.
        let host_non_sentinel = "\"\u{F0800}\"";
        assert_eq!(
            scan(host_non_sentinel, JsonSourceMode::HostUtf8),
            JsonStringScan::Direct {
                start: 1,
                end: host_non_sentinel.len() - 1
            }
        );
    }

    #[test]
    fn unescaped_string_fast_path_defers_to_the_existing_decoder() {
        let escaped = r#""line\nfeed""#;
        assert_eq!(
            scan(escaped, JsonSourceMode::RuntimeWtf16),
            JsonStringScan::Decode {
                content_start: 1,
                resume: escaped.find('\\').unwrap(),
                potential_host_sentinel: false
            }
        );
        let control = "\"bad\u{0001}\"";
        assert_eq!(
            scan(control, JsonSourceMode::RuntimeWtf16),
            JsonStringScan::Decode {
                content_start: 1,
                resume: control.find('\u{0001}').unwrap(),
                potential_host_sentinel: false
            }
        );
        let unterminated = "\"unterminated";
        assert_eq!(
            scan(unterminated, JsonSourceMode::RuntimeWtf16),
            JsonStringScan::Decode {
                content_start: 1,
                resume: unterminated.len(),
                potential_host_sentinel: false
            }
        );
        let host_sentinel = "\"\u{F0000}\"";
        assert_eq!(
            scan(host_sentinel, JsonSourceMode::HostUtf8),
            JsonStringScan::Decode {
                content_start: 1,
                resume: 1,
                potential_host_sentinel: true
            }
        );
    }

    #[test]
    fn string_decoder_resumes_at_tail_errors_without_rescanning_prefix() {
        let env = test_env();
        let trailing_escape = "\"long scanned prefix\\";
        let mut parser = JsonParser::new(trailing_escape, &env, JsonSourceMode::RuntimeWtf16);
        assert!(parser.string_value_rc().is_err());
        assert_eq!(parser.cursor, trailing_escape.len());

        let unterminated = format!("\"{}", "x".repeat(4096));
        assert_eq!(
            scan(&unterminated, JsonSourceMode::RuntimeWtf16),
            JsonStringScan::Decode {
                content_start: 1,
                resume: unterminated.len(),
                potential_host_sentinel: false
            }
        );
        let mut parser = JsonParser::new(&unterminated, &env, JsonSourceMode::RuntimeWtf16);
        assert!(parser.string_value_rc().is_err());
        assert_eq!(parser.cursor, unterminated.len());
    }

    #[test]
    fn unescaped_string_fast_path_preserves_parsed_keys_and_values() {
        let env = test_env();
        let value =
            parse_json_text(r#"{"文😀":"plain value"}"#, &env).expect("unescaped object parses");
        let Value::Object(value) = value else {
            panic!("object expected");
        };
        assert_eq!(
            value.get("文😀"),
            Some(Value::String(Rc::new("plain value".to_owned())))
        );
    }

    #[test]
    fn metadata_is_retained_only_for_the_reviver_parse_mode() {
        let source = r#"{"items":[1,"x",true,null]}"#;
        let env = test_env();

        let plain = JsonParser::new(source, &env, JsonSourceMode::RuntimeWtf16)
            .parse::<false>()
            .unwrap();
        assert!(matches!(plain.children, JsonNodeChildren::None));

        let reviver = JsonParser::new(source, &env, JsonSourceMode::RuntimeWtf16)
            .parse::<true>()
            .unwrap();
        let JsonNodeChildren::Object(properties) = reviver.children else {
            panic!("reviver mode must retain object children");
        };
        let JsonNodeChildren::Array(items) = &properties[0].1.children else {
            panic!("reviver mode must retain array children");
        };
        assert_eq!(items[0].source.as_deref(), Some("1"));
        assert_eq!(items[1].source.as_deref(), Some("\"x\""));
        assert_eq!(items[2].source.as_deref(), Some("true"));
        assert_eq!(items[3].source.as_deref(), Some("null"));
    }

    #[test]
    fn distinguishes_host_utf8_json_from_runtime_wtf16_json() {
        let env = test_env();
        let host = parse_host_json_text("\"\u{F0000}\"", &env).expect("host JSON parses");
        let Value::String(host) = host else {
            panic!("host JSON string expected");
        };
        assert_eq!(
            crate::string::string_code_units(&host),
            vec![0xDB80, 0xDC00]
        );

        let high = crate::string::char_from_code_unit(0xDB80);
        let low = crate::string::char_from_code_unit(0xDC00);
        let lone = crate::string::char_from_code_unit(0xD800);
        let source = format!("\"{high}{low}{lone}\"");
        let runtime = parse_json_text(&source, &env).expect("runtime JSON parses");
        let Value::String(runtime) = runtime else {
            panic!("runtime JSON string expected");
        };
        assert_eq!(
            crate::string::string_code_units(&runtime),
            vec![0xDB80, 0xDC00, 0xD800]
        );
    }
}
