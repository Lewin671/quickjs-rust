//! Canonical builders for the runtime's UTF-8 sentinel representation.
//!
//! Inputs are already canonical stored text. Only a concatenation seam can
//! create a new surrogate pair. Real U+F0000..F07FF scalars remain two sentinels
//! to distinguish them from lone UTF-16 surrogates.

const SURROGATE_ESCAPE_SENTINEL_BASE: u32 = 0xF0000;

/// Decodes a stored lone-surrogate sentinel, not a host Unicode scalar.
pub fn surrogate_escape_code_unit(character: char) -> Option<u16> {
    let code = character as u32;
    if (SURROGATE_ESCAPE_SENTINEL_BASE..SURROGATE_ESCAPE_SENTINEL_BASE + 0x800).contains(&code) {
        Some((0xD800 + code - SURROGATE_ESCAPE_SENTINEL_BASE) as u16)
    } else {
        None
    }
}

fn merged_pair(high: char, low: char) -> Option<char> {
    let high = surrogate_escape_code_unit(high)?;
    let low = surrogate_escape_code_unit(low)?;
    if !(0xD800..=0xDBFF).contains(&high) || !(0xDC00..=0xDFFF).contains(&low) {
        return None;
    }
    let scalar = 0x10000 + ((u32::from(high) - 0xD800) << 10) + u32::from(low) - 0xDC00;
    if (0xF0000..=0xF07FF).contains(&scalar) {
        None
    } else {
        char::from_u32(scalar)
    }
}

/// Checks the storage invariant; intended for debug assertions, never hot paths.
pub fn is_canonical(text: &str) -> bool {
    let mut chars = text.chars();
    let Some(mut previous) = chars.next() else {
        return true;
    };
    for next in chars {
        if merged_pair(previous, next).is_some() {
            return false;
        }
        previous = next;
    }
    true
}

/// Appends canonical stored text, checking at most the two seam scalars.
/// The ASCII path inspects only the first suffix byte; no prefix scan occurs.
// The allocation/copy tail also stays shared rather than enlarging each
// runtime caller's inline graph. ASCII still needs no whole-string scan.
#[inline(never)]
pub fn push_js_str(output: &mut String, suffix: &str) {
    if suffix.as_bytes().first().is_some_and(|byte| *byte >= 0xF0) {
        push_wide_seam(output, suffix);
    } else {
        output.push_str(suffix);
    }
}

// Keep UTF-8 decoding and the pair-repair branch out of append callers' inline
// graph. Their ordinary path needs only the first suffix byte and bulk copy.
#[cold]
#[inline(never)]
fn push_wide_seam(output: &mut String, suffix: &str) {
    if let Some(last) = output.chars().next_back()
        && let Some(first) = suffix.chars().next()
        && let Some(merged) = merged_pair(last, first)
    {
        output.truncate(output.len() - last.len_utf8());
        output.push(merged);
        output.push_str(&suffix[first.len_utf8()..]);
    } else {
        output.push_str(suffix);
    }
}

/// Appends one stored scalar (including a lone-surrogate sentinel).
#[inline]
pub fn push_js_char(output: &mut String, character: char) {
    // Only a low-surrogate sentinel can merge with the existing tail.
    if !('\u{F0400}'..='\u{F07FF}').contains(&character) {
        output.push(character);
    } else {
        push_js_str(output, character.encode_utf8(&mut [0; 4]));
    }
}

/// Repeats stored text, repairing only repetition seams. Ordinary strings
/// retain Rust's bulk repeat path; no per-character classification is added.
pub fn repeat_js_str(text: &str, count: usize) -> String {
    if count <= 1
        || text
            .chars()
            .next_back()
            .zip(text.chars().next())
            .is_none_or(|(last, first)| merged_pair(last, first).is_none())
    {
        return text.repeat(count);
    }
    let mut output = String::with_capacity(text.len() * count);
    for _ in 0..count {
        push_js_str(&mut output, text);
    }
    output
}
