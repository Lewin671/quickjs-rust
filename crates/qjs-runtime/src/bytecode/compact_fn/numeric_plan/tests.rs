use super::{BOOLEAN, NUMBER, NumOp, from_bytecode};
use crate::bytecode::{compiler, ir::Bytecode, ir::Op};
use crate::{Value, eval};

fn nested_function(source: &str, name: &str) -> Bytecode {
    let script = qjs_parser::parse_script(source).expect("source should parse");
    let bytecode = compiler::compile_script(&script).expect("source should compile");
    bytecode
        .code
        .iter()
        .find_map(|op| match op {
            Op::NewFunction {
                name: actual,
                bytecode,
                ..
            } if actual.as_deref() == Some(name) => Some(bytecode.as_ref().clone()),
            _ => None,
        })
        .expect("named function bytecode should be nested in the script")
}

const HASHING: &str = "
    function computeHashCode(key) {
        switch (typeof key) {
        case 'undefined': return 0;
        case 'object': if (!key) return 0;
        case 'function': return key.hashCode();
        case 'boolean': return key | 0;
        case 'number':
            if ((key | 0) == key) return key;
            key = '' + key;
        case 'string':
            var h = 0, len = key.length;
            for (var index = 0; index < len; index++) h = (((31 * h) | 0) + key.charCodeAt(index)) | 0;
            return h;
        default: throw new Error('bad');
        }
    }
    function equals(a, b) {
        if (typeof a != typeof b) return false;
        switch (typeof a) {
        case 'object': if (!a) return !b;
        case 'function':
            switch (typeof b) { case 'object': case 'function': return a.equals(b); default: return false; }
        default: return a == b;
        }
    }
    function mixed(a) { return a > 1 ? a : a > 0; }";

/// Under number arguments every `typeof` test folds, so what is left of a
/// hash table's key functions is a few numeric operations, a return, and a
/// hand-back where a non-integer key would be converted to a string.
#[test]
fn typeof_switches_fold_to_their_number_case() {
    let hash = from_bytecode::lower(&nested_function(HASHING, "computeHashCode"))
        .expect("the number case should lower");
    assert_eq!(hash.returns, NUMBER);
    assert!(hash.ops.iter().any(|op| matches!(op, NumOp::Bail)));
    assert!(!hash.calls_out());
    let equals = from_bytecode::lower(&nested_function(HASHING, "equals"))
        .expect("the default case should lower");
    assert_eq!(equals.returns, BOOLEAN);
    assert!(
        from_bytecode::lower(&nested_function(HASHING, "mixed")).is_none(),
        "a body returning numbers and booleans has no single result kind"
    );
}

#[test]
fn bytecode_plans_agree_with_the_interpreter() {
    let source = "
        function kind(x) { switch (typeof x) { case 'number': return (x | 0) == x ? 1 : 2; case 'string': return 3; default: return 4; } }
        function sum(n) { var s = 0; for (var i = 0; i < n; i++) s += i; return s; }
        function either(a, b) { return a || b; }
        function both(a, b) { return a && b; }
        function isBig(x) { return typeof x === 'number' && x > 3; }
        function viaBig(x) { return isBig(x) + 1; }
        function dead(a) { if (a > 0) { return g; } let g = 2; return a + g; }
        function neg(a) { return -a + ~a + !a; }
        function down(n) { var k = n; while (k-- > 0) {} return k; }
        function run(n) {
            var out = [0, 0, 0, 0, 0, 0, 0, 0, 0], errors = 0;
            for (var i = 0; i < n; i++) {
                out[0] += kind(i) + kind(i + 0.5);
                out[1] += sum(i & 15);
                out[2] += either(i & 1, 7) + both(i & 1, 7);
                out[3] += isBig(i & 7) ? 1 : 0;
                out[4] += viaBig(i & 7);
                try { out[5] += dead(i & 1); } catch (e) { if (e instanceof ReferenceError) errors++; }
                out[6] += neg(i & 3);
                out[7] += down(i & 3);
            }
            out[8] = [kind('x'), kind(null), either(0, 0), both(0, 5), isBig('9'), neg(0), 1 / neg(-0)].join(':');
            return out.join() + '/' + errors;
        }
        run(200);";
    assert_eq!(
        eval(source),
        Ok(Value::String(
            "600,6776,1500,100,300,200,-750,-200,3:4:0:0:false:0:Infinity/100".into()
        ))
    );
}

#[test]
fn hash_keys_run_on_plans_and_hand_back_everything_else() {
    let source = format!(
        "{HASHING}
        function run(n) {{
            var s = 0, e = 0;
            for (var i = 0; i < n; i++) {{ s = (s + computeHashCode(i * 7)) | 0; if (equals(i & 3, 2)) e++; }}
            return [s, e, computeHashCode(1.5), computeHashCode('ab'), computeHashCode(true),
                computeHashCode(undefined), computeHashCode(null), equals(1, 1), equals(1, 2),
                equals(1, '1'), equals(NaN, NaN), equals(null, null), equals(0, -0),
                1 / computeHashCode(-0)].join();
        }}
        run(1000);"
    );
    assert_eq!(
        eval(&source),
        Ok(Value::String(
            "3496500,250,48568,3105,1,0,0,true,false,false,false,true,true,-Infinity".into()
        ))
    );
}
