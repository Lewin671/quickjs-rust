use qjs_ast::{AssignmentTarget, Expr, Literal, Stmt, VarKind};
use qjs_lexer::TokenKind;

use crate::ParseError;

pub(crate) fn property_name(kind: TokenKind) -> Option<String> {
    match kind {
        TokenKind::Identifier(name) => Some(name),
        TokenKind::True => Some("true".to_owned()),
        TokenKind::False => Some("false".to_owned()),
        TokenKind::Null => Some("null".to_owned()),
        TokenKind::This => Some("this".to_owned()),
        TokenKind::Var => Some("var".to_owned()),
        TokenKind::Let => Some("let".to_owned()),
        TokenKind::Const => Some("const".to_owned()),
        TokenKind::If => Some("if".to_owned()),
        TokenKind::Else => Some("else".to_owned()),
        TokenKind::While => Some("while".to_owned()),
        TokenKind::Do => Some("do".to_owned()),
        TokenKind::For => Some("for".to_owned()),
        TokenKind::Switch => Some("switch".to_owned()),
        TokenKind::Case => Some("case".to_owned()),
        TokenKind::Default => Some("default".to_owned()),
        TokenKind::Try => Some("try".to_owned()),
        TokenKind::Catch => Some("catch".to_owned()),
        TokenKind::Finally => Some("finally".to_owned()),
        TokenKind::Break => Some("break".to_owned()),
        TokenKind::Continue => Some("continue".to_owned()),
        TokenKind::Function => Some("function".to_owned()),
        TokenKind::Class => Some("class".to_owned()),
        TokenKind::Extends => Some("extends".to_owned()),
        TokenKind::Super => Some("super".to_owned()),
        TokenKind::Return => Some("return".to_owned()),
        TokenKind::Throw => Some("throw".to_owned()),
        TokenKind::Debugger => Some("debugger".to_owned()),
        TokenKind::Typeof => Some("typeof".to_owned()),
        TokenKind::Void => Some("void".to_owned()),
        TokenKind::In => Some("in".to_owned()),
        TokenKind::With => Some("with".to_owned()),
        TokenKind::Delete => Some("delete".to_owned()),
        TokenKind::New => Some("new".to_owned()),
        TokenKind::Instanceof => Some("instanceof".to_owned()),
        _ => None,
    }
}

/// Reports whether a decoded IdentifierName is forbidden as an Identifier in
/// ordinary script contexts. Escaped spellings reach the parser as Identifier
/// tokens so property-name grammar can still accept them.
pub(crate) fn is_reserved_identifier_name(name: &str) -> bool {
    matches!(
        name,
        "true"
            | "false"
            | "null"
            | "this"
            | "var"
            | "const"
            | "if"
            | "else"
            | "while"
            | "do"
            | "for"
            | "switch"
            | "case"
            | "default"
            | "try"
            | "catch"
            | "finally"
            | "break"
            | "continue"
            | "function"
            | "class"
            | "extends"
            | "super"
            | "return"
            | "throw"
            | "debugger"
            | "typeof"
            | "void"
            | "in"
            | "with"
            | "delete"
            | "new"
            | "instanceof"
            | "enum"
    )
}

/// Computes the `PropName` of a numeric-literal property key.
///
/// The spec defines the property name of a `NumericLiteral` member as
/// `ToString(MV)` of the literal, so `0b10`, `0x10`, and `1.0` name the
/// properties `"2"`, `"16"`, and `"1"`. This is a static (lexical) semantic,
/// not runtime evaluation: it depends only on the literal text.
pub(crate) fn numeric_property_key(raw: &str) -> String {
    let cleaned: String = raw.chars().filter(|&ch| ch != '_').collect();
    let value = if let Some(digits) = cleaned
        .strip_prefix("0x")
        .or_else(|| cleaned.strip_prefix("0X"))
    {
        radix_value(digits, 16)
    } else if let Some(digits) = cleaned
        .strip_prefix("0b")
        .or_else(|| cleaned.strip_prefix("0B"))
    {
        radix_value(digits, 2)
    } else if let Some(digits) = cleaned
        .strip_prefix("0o")
        .or_else(|| cleaned.strip_prefix("0O"))
    {
        radix_value(digits, 8)
    } else if cleaned.len() > 1
        && cleaned.starts_with('0')
        && cleaned.bytes().all(|b| b.is_ascii_digit())
    {
        // Legacy octal (`0777`) is only reached in non-strict code; the lexer
        // accepts the token, so honor the octal interpretation here.
        radix_value(&cleaned[1..], 8)
    } else {
        cleaned.parse::<f64>().ok()
    };
    match value {
        Some(number) => number_to_property_string(number),
        // Fall back to the raw text if parsing somehow fails; the runtime would
        // surface any genuinely malformed literal.
        None => raw.to_owned(),
    }
}

/// Computes the `PropName` of a BigInt literal property key.
pub(crate) fn bigint_property_key(raw: &str) -> String {
    let cleaned: String = raw.chars().filter(|&ch| ch != '_').collect();
    let (digits, radix) = if let Some(digits) = cleaned
        .strip_prefix("0x")
        .or_else(|| cleaned.strip_prefix("0X"))
    {
        (digits, 16)
    } else if let Some(digits) = cleaned
        .strip_prefix("0b")
        .or_else(|| cleaned.strip_prefix("0B"))
    {
        (digits, 2)
    } else if let Some(digits) = cleaned
        .strip_prefix("0o")
        .or_else(|| cleaned.strip_prefix("0O"))
    {
        (digits, 8)
    } else {
        (cleaned.as_str(), 10)
    };
    integer_digits_to_decimal(digits, radix)
}

fn integer_digits_to_decimal(digits: &str, radix: u32) -> String {
    if digits.is_empty() {
        return "0".to_owned();
    }
    let mut decimal = vec![0_u8];
    for ch in digits.chars() {
        let Some(value) = ch.to_digit(radix) else {
            return digits.to_owned();
        };
        let mut carry = value;
        for digit in decimal.iter_mut().rev() {
            let next = u32::from(*digit) * radix + carry;
            *digit = (next % 10) as u8;
            carry = next / 10;
        }
        while carry > 0 {
            decimal.insert(0, (carry % 10) as u8);
            carry /= 10;
        }
    }
    let first_non_zero = decimal
        .iter()
        .position(|digit| *digit != 0)
        .unwrap_or(decimal.len() - 1);
    decimal[first_non_zero..]
        .iter()
        .map(|digit| char::from(b'0' + *digit))
        .collect()
}

fn radix_value(digits: &str, radix: u32) -> Option<f64> {
    if digits.is_empty() {
        return None;
    }
    let mut value = 0.0;
    for ch in digits.chars() {
        let digit = ch.to_digit(radix)?;
        value = value * f64::from(radix) + f64::from(digit);
    }
    Some(value)
}

/// Mirrors the runtime `Number` ToString used for property keys. Kept local so
/// the parser stays free of an engine dependency; the integer fast path covers
/// every numeric property key Test262 exercises and matches the runtime.
fn number_to_property_string(number: f64) -> String {
    if number == 0.0 {
        return "0".to_owned();
    }
    // Exact integers within the i64 range stringify without a fractional tail.
    if number.is_finite() && number.fract() == 0.0 && number.abs() < 9.007_199_254_740_992e15 {
        return format!("{}", number as i64);
    }
    if number.abs() >= 1e21 || number.abs() < 1e-6 {
        let formatted = format!("{number:e}");
        if let Some((mantissa, exponent)) = formatted.split_once('e') {
            let mantissa = mantissa.trim_end_matches('0').trim_end_matches('.');
            let exponent = if let Some(unsigned) = exponent.strip_prefix('-') {
                format!("-{}", unsigned.trim_start_matches('0'))
            } else {
                format!("+{}", exponent.trim_start_matches('0'))
            };
            return format!("{mantissa}e{exponent}");
        }
        return formatted;
    }
    number.to_string()
}

pub(crate) fn var_kind(kind: &TokenKind) -> Option<VarKind> {
    match kind {
        TokenKind::Var => Some(VarKind::Var),
        TokenKind::Let => Some(VarKind::Let),
        TokenKind::Const => Some(VarKind::Const),
        _ => None,
    }
}

pub(crate) fn assignment_target(
    expr: Expr,
    parenthesized: bool,
    strict: bool,
) -> Result<AssignmentTarget, ParseError> {
    match expr {
        Expr::Identifier { name, span } => Ok(AssignmentTarget::Identifier {
            name,
            span,
            parenthesized,
        }),
        Expr::Member {
            object,
            property,
            span,
        } => Ok(AssignmentTarget::Member {
            object,
            property,
            span,
        }),
        // AnnexB web compatibility: a CallExpression (`f()`, `async()`) is a
        // valid assignment/update target in sloppy mode — it is evaluated and a
        // runtime ReferenceError is thrown. Strict mode keeps the early error.
        call @ Expr::Call { .. } if !strict => {
            let span = call.span();
            Ok(AssignmentTarget::CallExpression {
                call: Box::new(call),
                span,
            })
        }
        other => Err(ParseError {
            message: "invalid assignment target".to_owned(),
            span: other.span(),
        }),
    }
}

pub(crate) fn stmt_end(stmt: &Stmt) -> usize {
    match stmt {
        Stmt::Expr(expr) => expr.span().end,
        Stmt::Block { span, .. }
        | Stmt::If { span, .. }
        | Stmt::While { span, .. }
        | Stmt::DoWhile { span, .. }
        | Stmt::For { span, .. }
        | Stmt::ForIn { span, .. }
        | Stmt::ForOf { span, .. }
        | Stmt::Switch { span, .. }
        | Stmt::Try { span, .. }
        | Stmt::FunctionDecl { span, .. }
        | Stmt::ClassDecl { span, .. }
        | Stmt::Labelled { span, .. }
        | Stmt::Return { span, .. }
        | Stmt::Throw { span, .. }
        | Stmt::Debugger { span }
        | Stmt::Break { span, .. }
        | Stmt::Continue { span, .. }
        | Stmt::VarDecl { span, .. }
        | Stmt::With { span, .. } => span.end,
        Stmt::ModuleDecl(decl) => decl.span().end,
        Stmt::Empty => 0,
    }
}

pub(crate) fn body_has_strict_directive(body: &[Stmt]) -> bool {
    for stmt in body {
        let Stmt::Expr(Expr::Literal(Literal::String { value, span })) = stmt else {
            return false;
        };
        // A Use Strict Directive is determined by source text, not value: an
        // escape or line continuation (e.g. `'use str\<LF>ict'`) computes to
        // "use strict" but is not the directive. Such a literal spans more than
        // the bare `'use strict'` (12 bytes), so the span length disambiguates.
        if value == "use strict" && span.end - span.start == 12 {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::{bigint_property_key, numeric_property_key};

    #[test]
    fn canonicalizes_numeric_property_keys() {
        assert_eq!(numeric_property_key("0b10"), "2");
        assert_eq!(numeric_property_key("0B10"), "2");
        assert_eq!(numeric_property_key("0x10"), "16");
        assert_eq!(numeric_property_key("0o17"), "15");
        assert_eq!(numeric_property_key("0777"), "511");
        assert_eq!(numeric_property_key("100"), "100");
        assert_eq!(numeric_property_key("1e3"), "1000");
        assert_eq!(numeric_property_key("1.5"), "1.5");
        assert_eq!(numeric_property_key(".5"), "0.5");
        assert_eq!(numeric_property_key("1.0"), "1");
        assert_eq!(numeric_property_key("0"), "0");
        assert_eq!(numeric_property_key("1_000"), "1000");
        assert_eq!(numeric_property_key("0x1_0"), "16");
        assert_eq!(numeric_property_key("1e-7"), "1e-7");
        assert_eq!(numeric_property_key("1e21"), "1e+21");
    }

    #[test]
    fn canonicalizes_bigint_property_keys() {
        assert_eq!(bigint_property_key("0"), "0");
        assert_eq!(bigint_property_key("1_000"), "1000");
        assert_eq!(bigint_property_key("0xf"), "15");
        assert_eq!(bigint_property_key("0b101"), "5");
        assert_eq!(bigint_property_key("0o77"), "63");
        assert_eq!(
            bigint_property_key("999999999999999999"),
            "999999999999999999"
        );
    }
}
