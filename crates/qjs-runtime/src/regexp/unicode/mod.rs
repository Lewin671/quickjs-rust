//! Unicode property-escape resolution for `\p{...}` / `\P{...}`.
//!
//! The range tables under this module are generated from the Unicode
//! Character Database 17.0.0 (see the `GENERATED FILE` headers and `tables.rs`).
//! Resolution follows the ECMAScript `RegExp` grammar
//! (sec-static-semantics-unicodematchproperty-p and friends): property and
//! value names match the canonical Unicode names and their listed aliases
//! exactly, with no loose matching.

mod aliases;
mod tables;

/// A resolved property escape: a sorted, non-overlapping set of code-point
/// ranges, optionally negated for `\P{...}`.
///
/// `Copy` so callers can resolve a property once at match setup and cheaply
/// carry the static range slice through the per-character matching loop.
#[derive(Clone, Copy)]
pub(crate) struct PropertySet {
    ranges: &'static [(u32, u32)],
}

impl PropertySet {
    pub(crate) fn contains(&self, code_point: u32) -> bool {
        self.ranges
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
}

/// Resolve a `\p{...}` body (the text between the braces) into a property set.
///
/// Returns `None` when the body is not a valid Unicode property expression, in
/// which case the regular expression is a SyntaxError.
pub(crate) fn resolve_property(body: &str) -> Option<PropertySet> {
    let ranges = if let Some((name, value)) = body.split_once('=') {
        resolve_property_value(name, value)?
    } else {
        resolve_lone(body)?
    };
    Some(PropertySet { ranges })
}

fn resolve_property_value(name: &str, value: &str) -> Option<&'static [(u32, u32)]> {
    // No loose matching: names/values must not carry surrounding whitespace.
    if has_extra_whitespace(name) || has_extra_whitespace(value) || value.is_empty() {
        return None;
    }
    match canonical_property_name(name) {
        "General_Category" => gc_value_ranges(value),
        "Script" => {
            let canon = aliases::script_value_alias(value)?;
            tables::script_ranges(canon)
        }
        "Script_Extensions" => {
            // The `scx_*` tables store the complete Script_Extensions set for
            // each script (the script's own code points unioned with the
            // points whose explicit Script_Extensions list names it); for
            // `Common`/`Inherited` the points reassigned to specific scripts
            // are removed. Resolution is therefore a single table lookup.
            let canon = aliases::script_value_alias(value)?;
            tables::script_ext_ranges(canon)
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

#[cfg(test)]
mod tests;
