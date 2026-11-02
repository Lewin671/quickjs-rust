//! Target selection and observable statement-list completion on abrupt exits.
use crate::{Value, eval};

#[test]
fn unlabeled_break_skips_label_only_contexts() {
    assert_eq!(
        eval(include_str!(
            "../../../../tests/fixtures/compare-qjs/unlabeled-break-through-labels.js"
        )),
        Ok(Value::String("w1,end|f0|a0,a1,a2|switch".to_owned().into()))
    );
    assert!(qjs_parser::parse_script("L:{break;}").is_err());
}
