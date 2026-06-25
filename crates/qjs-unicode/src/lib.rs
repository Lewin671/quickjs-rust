//! Unicode property and identifier tables shared by the lexer and runtime.
//!
//! The range tables under this module are generated from the Unicode
//! Character Database 17.0.0 (see the `GENERATED FILE` headers and `tables.rs`).
//! Resolution follows the ECMAScript `RegExp` grammar
//! (sec-static-semantics-unicodematchproperty-p and friends): property and
//! value names match the canonical Unicode names and their listed aliases
//! exactly, with no loose matching.

mod aliases;
mod tables;

use std::sync::OnceLock;

/// A resolved property escape: a sorted, non-overlapping set of code-point
/// ranges, optionally negated for `\P{...}`.
///
/// `Copy` so callers can resolve a property once at match setup and cheaply
/// carry the static range slice through the per-character matching loop.
#[derive(Clone, Copy)]
pub enum PropertySet {
    Ranges(&'static [(u32, u32)]),
    ScriptUnknown,
}

impl PropertySet {
    pub fn contains(&self, code_point: u32) -> bool {
        match self {
            Self::Ranges(ranges) => ranges_contain(ranges, code_point),
            Self::ScriptUnknown => script_unknown_contains(code_point),
        }
    }
}

/// Resolve a `\p{...}` body (the text between the braces) into a property set.
///
/// Returns `None` when the body is not a valid Unicode property expression, in
/// which case the regular expression is a SyntaxError.
pub fn resolve_property(body: &str) -> Option<PropertySet> {
    if let Some((name, value)) = body.split_once('=') {
        resolve_property_value(name, value)
    } else {
        resolve_lone(body).map(PropertySet::Ranges)
    }
}

fn resolve_property_value(name: &str, value: &str) -> Option<PropertySet> {
    // No loose matching: names/values must not carry surrounding whitespace.
    if has_extra_whitespace(name) || has_extra_whitespace(value) || value.is_empty() {
        return None;
    }
    match canonical_property_name(name) {
        "General_Category" => gc_value_ranges(value).map(PropertySet::Ranges),
        "Script" => {
            let canon = aliases::script_value_alias(value)?;
            if canon == "Unknown" {
                Some(PropertySet::ScriptUnknown)
            } else {
                tables::script_ranges(canon).map(PropertySet::Ranges)
            }
        }
        "Script_Extensions" => {
            // The `scx_*` tables store the complete Script_Extensions set for
            // each script (the script's own code points unioned with the
            // points whose explicit Script_Extensions list names it); for
            // `Common`/`Inherited` the points reassigned to specific scripts
            // are removed. Resolution is therefore a single table lookup.
            let canon = aliases::script_value_alias(value)?;
            if canon == "Unknown" {
                Some(PropertySet::ScriptUnknown)
            } else {
                tables::script_ext_ranges(canon).map(PropertySet::Ranges)
            }
        }
        _ => None,
    }
}

/// Canonicalize the property *name* used on the left side of `name=value`.
fn canonical_property_name(name: &str) -> &str {
    match name {
        "General_Category" | "gc" => "General_Category",
        "Script" | "sc" => "Script",
        "Script_Extensions" | "scx" => "Script_Extensions",
        other => other,
    }
}

fn resolve_lone(name: &str) -> Option<&'static [(u32, u32)]> {
    if has_extra_whitespace(name) || name.is_empty() {
        return None;
    }
    // A lone name is either a binary property name or a General_Category value.
    if let Some(ranges) = binary_lookup(name) {
        return Some(ranges);
    }
    gc_value_ranges(name)
}

/// Resolve a General_Category value (canonical name, short or long alias) to
/// its range table.
fn gc_value_ranges(value: &str) -> Option<&'static [(u32, u32)]> {
    // `gc_value_alias` covers short/long/extra aliases; canonical table keys
    // (including the group values like `L`) are also accepted directly.
    if let Some(canon) = aliases::gc_value_alias(value) {
        return tables::gc_ranges(canon);
    }
    tables::gc_ranges(value)
}

/// Look up a binary property by canonical name or alias.
fn binary_lookup(name: &str) -> Option<&'static [(u32, u32)]> {
    if let Some(ranges) = tables::binary_ranges(name) {
        return Some(ranges);
    }
    let canon = aliases::property_alias(name)?;
    tables::binary_ranges(canon)
}

fn has_extra_whitespace(text: &str) -> bool {
    text != text.trim() || text.is_empty()
}

/// True if `code_point` carries the Unicode `ID_Start` property. The RegExp
/// `GroupName` grammar (`RegExpIdentifierName`) is defined in terms of
/// `ID_Start`/`ID_Continue` independently of the `u` flag.
pub fn is_id_start(code_point: u32) -> bool {
    tables::binary_ranges("ID_Start").is_some_and(|ranges| ranges_contain(ranges, code_point))
}

/// True if `code_point` carries the Unicode `ID_Continue` property.
pub fn is_id_continue(code_point: u32) -> bool {
    tables::binary_ranges("ID_Continue").is_some_and(|ranges| ranges_contain(ranges, code_point))
}

fn ranges_contain(ranges: &[(u32, u32)], code_point: u32) -> bool {
    ranges
        .binary_search_by(|&(start, end)| {
            if code_point < start {
                std::cmp::Ordering::Greater
            } else if code_point > end {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Equal
            }
        })
        .is_ok()
}

fn script_unknown_contains(code_point: u32) -> bool {
    static CN: OnceLock<&'static [(u32, u32)]> = OnceLock::new();
    static CO: OnceLock<&'static [(u32, u32)]> = OnceLock::new();
    let cn = *CN.get_or_init(|| tables::gc_ranges("Cn").expect("Cn table must exist"));
    let co = *CO.get_or_init(|| tables::gc_ranges("Co").expect("Co table must exist"));
    (0xD800..=0xDFFF).contains(&code_point)
        || ranges_contain(cn, code_point)
        || ranges_contain(co, code_point)
}

#[cfg(test)]
mod tests;
