use super::*;
use crate::{Value, eval};

#[test]
fn ordinary_arrays_keep_bitwise_chains_in_the_compact_word_lane() {
    dense::reset_test_iterations();
    let result = eval(
        r#"
function convert(values, signed, unsigned) {
  for (var i = 0; i < values.length; i = i + 1) {
    signed[i] = values[i] | 0;
    unsigned[i] = values[i] >>> 0;
  }
  return signed.join(':') + '|' + unsigned.join(':');
}
convert(
  [-1, 2147483648, 4294967297, NaN],
  [0, 0, 0, 0],
  [0, 0, 0, 0]
);
"#,
    );
    assert_eq!(
        result,
        Ok(Value::String(
            "-1:-2147483648:1:0|4294967295:2147483648:1:0"
                .to_owned()
                .into()
        ))
    );
    assert!(dense::test_compact_word_iterations() > 0);
    assert!(dense::test_writable_path_hits() > 0);
}

#[test]
fn ordinary_array_word_lane_preserves_sunk_store_and_mid_iteration_replay() {
    dense::reset_test_iterations();
    let result = eval(
        r#"
var coercions = 0;
var marker = { valueOf: function () { coercions = coercions + 1; return 7; } };
function mutate(input, output) {
  for (var i = 0; i < input.length; i = i + 1) {
    output[i] = (input[i] ^ (i << 1)) >>> 0;
  }
  return output.join(':') + '|' + coercions;
}
mutate([1, 2, marker, 4], [10, 20, 30, 40]);
"#,
    );
    assert_eq!(result, Ok(Value::String("1:0:3:2|1".to_owned().into())));
    assert!(dense::test_compact_word_iterations() > 0);
}
