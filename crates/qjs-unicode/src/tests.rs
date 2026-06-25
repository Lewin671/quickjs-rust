use super::resolve_property;

fn matches(body: &str, code_point: u32) -> bool {
    resolve_property(body)
        .expect("property should resolve")
        .contains(code_point)
}

#[test]
fn general_category_lone_and_qualified() {
    // Decimal_Number via short, long, alias, and qualified forms.
    for body in [
        "Nd",
        "Decimal_Number",
        "gc=Nd",
        "gc=digit",
        "General_Category=Decimal_Number",
    ] {
        assert!(matches(body, 0x30), "`{body}` should match '0'");
        assert!(!matches(body, 0x41), "`{body}` should not match 'A'");
    }
}

#[test]
fn general_category_groups() {
    assert!(matches("L", 0x41)); // letter
    assert!(matches("Letter", 0x41));
    assert!(!matches("L", 0x30));
    assert!(matches("N", 0x30)); // number
    assert!(matches("P", '!' as u32)); // punctuation
}

#[test]
fn binary_properties() {
    assert!(matches("ASCII", 0x41));
    assert!(!matches("ASCII", 0x100));
    assert!(matches("Any", 0x10FFFF));
    assert!(matches("White_Space", 0x20));
    assert!(matches("Alphabetic", 0x41));
    assert!(matches("Alpha", 0x41)); // alias
    assert!(matches("Hex_Digit", 0x46));
}

#[test]
fn scripts_and_extensions() {
    assert!(matches("Script=Latin", 0x41));
    assert!(matches("sc=Latn", 0x41)); // script alias
    assert!(matches("Script=Greek", 0x3B1));
    assert!(matches("Script_Extensions=Latin", 0x41));
    assert!(matches("scx=Grek", 0x3B1));
}

#[test]
fn unknown_script_covers_unassigned_private_use_and_surrogates() {
    assert!(matches("Script=Unknown", 0x038B)); // unassigned
    assert!(matches("Script=Zzzz", 0xE000)); // private-use
    assert!(matches("sc=Unknown", 0xD800)); // surrogate
    assert!(matches("Script_Extensions=Unknown", 0x038B));
    assert!(matches("scx=Zzzz", 0xE000));
    assert!(!matches("Script=Unknown", 0x41));
    assert!(!matches("Script_Extensions=Unknown", 0x41));
}

#[test]
fn rejects_invalid() {
    // Loose matching (surrounding whitespace) is not allowed.
    assert!(resolve_property(" ASCII ").is_none());
    assert!(resolve_property("General_Category = Uppercase_Letter").is_none());
    // Non-binary property name used as a lone name.
    assert!(resolve_property("General_Category").is_none());
    assert!(resolve_property("Script").is_none());
    // Unsupported / non-existent properties.
    assert!(resolve_property("Hyphen").is_none());
    assert!(resolve_property("Other_Alphabetic").is_none());
    assert!(resolve_property("Line_Break").is_none());
    assert!(resolve_property("FooBar").is_none());
    assert!(resolve_property("gc=FooBar").is_none());
    assert!(resolve_property("Script=Combining").is_none());
    // Empty value.
    assert!(resolve_property("gc=").is_none());
}

#[test]
fn case_sensitive_names() {
    // Canonical names and listed aliases match; arbitrary case folding does not.
    assert!(resolve_property("ascii").is_none());
    assert!(resolve_property("DECIMAL_NUMBER").is_none());
}
