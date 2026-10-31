//! Observer-free JSON object construction, predicting sibling key sequences.
//! Escaped keys still use the complete decoder; only raw validated spans borrow.
use super::*;
use crate::value::layout::DataSequence;
impl JsonParser<'_> {
    pub(super) fn object_sequence(&mut self) -> Result<JsonNode, RuntimeError> {
        self.expect_byte(b'{')?;
        let depth = self.object_depth;
        self.object_depth += 1;
        if self.sequences.len() <= depth {
            self.sequences.resize_with(depth + 1, || None);
        }
        let mut properties = DataSequence::new(
            self.sequences[depth].take(),
            self.env.realm().atoms(),
            object_prototype(self.env).map(crate::Prototype::Object),
        );
        self.skip_whitespace();
        if !self.consume_byte(b'}') {
            loop {
                self.skip_whitespace();
                if self.peek_byte() != Some(b'"') {
                    return Err(self.syntax_error());
                }
                let key = match self.scan_string()? {
                    JsonStringScan::Direct { start, end } => {
                        self.cursor = end + 1;
                        std::borrow::Cow::Borrowed(&self.source[start..end])
                    }
                    JsonStringScan::Decode {
                        content_start,
                        resume,
                        potential_host_sentinel,
                    } => std::borrow::Cow::Owned(self.decode_string(
                        content_start,
                        resume,
                        potential_host_sentinel,
                    )?),
                };
                self.skip_whitespace();
                self.expect_byte(b':')?;
                let child = self.value::<false>()?;
                properties.insert(&key, child.value);
                self.skip_whitespace();
                if self.consume_byte(b'}') {
                    break;
                }
                self.expect_byte(b',')?;
            }
        }
        let (object, prediction) = OrderedDataPropertyBuilder::finish_sequence(properties);
        self.sequences[depth] = prediction;
        self.object_depth -= 1;
        Ok(empty_node(Value::Object(object)))
    }
}

#[cfg(test)]
mod tests {
    use crate::{Value, eval};
    #[test]
    fn sequence_mismatches_prefixes_duplicates_indices_and_nested_siblings() {
        assert_eq!(
            eval(
                r#"
            let text='[{"a":1,"b":{"x":1}},{"a":2,"b":{"x":2}},{"a":3},{"a":4,"a":5,"0":6},{"a":7,"b":{"y":8}},{"\\u0061":9,"b":10}]';
            let xs=JSON.parse(text);
            JSON.stringify(xs)==='[{"a":1,"b":{"x":1}},{"a":2,"b":{"x":2}},{"a":3},{"0":6,"a":5},{"a":7,"b":{"y":8}},{"a":9,"b":10}]'
                && Object.getPrototypeOf(xs[0])===Object.prototype;
        "#
            ),
            Ok(Value::Boolean(true))
        );
    }
    #[test]
    fn sequence_preserves_proto_data_and_rejects_malformed_matched_keys() {
        assert_eq!(
            eval(
                r#"
            let xs=JSON.parse('[{"__proto__":1,"x":2},{"__proto__":3,"x":4}]');
            let errors=0;for(let s of ['[{"x":1},{"x":2,}]','[{"x":1},{"x" 2}]','[{"x":1},{"x":2}'] ) {
                try {JSON.parse(s);}catch(e){if(e instanceof SyntaxError)errors++;}
            }
            xs[1].__proto__===3 && Object.getPrototypeOf(xs[1])===Object.prototype && errors===3;
        "#
            ),
            Ok(Value::Boolean(true))
        );
    }
}
