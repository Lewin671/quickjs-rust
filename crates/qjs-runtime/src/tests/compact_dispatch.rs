use crate::{Value, eval};

#[test]
fn compact_dispatch_preserves_numeric_edges_and_bigint_errors() {
    assert_eq!(
        eval(
            "function arithmetic(kind, left, right) { \
               if (kind === 0) return left / right; \
               if (kind === 1) return left + right; \
               return left - right; \
             } \
             var nan = arithmetic(0, 0, 0); \
             var negativeZero = arithmetic(0, -0, 1); \
             var infinite = arithmetic(0, 1, 0); \
             var mixedBigInt = false; \
             try { arithmetic(1, 1n, 1); } \
             catch (error) { mixedBigInt = error instanceof TypeError; } \
             Number.isNaN(nan) && Object.is(negativeZero, -0) && \
               infinite === Infinity && mixedBigInt;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn compact_dispatch_preserves_unary_update_typeof_and_template_conversion() {
    assert_eq!(
        eval(
            "var log = ''; \
             function unaryOrUpdate(kind, value) { \
               if (kind === 0) return -value; \
               if (kind === 1) return typeof value; \
               return ++value; \
             } \
             function compactTemplate(value) { \
               if (value === null) return ''; \
               return `${value}`; \
             } \
             var object = { \
               toString: function() { log = log + 's'; return 'ok'; } \
             }; \
             Object.is(unaryOrUpdate(0, 0), -0) && \
               unaryOrUpdate(0, Infinity) === -Infinity && \
               unaryOrUpdate(1, 1n) === 'bigint' && \
               unaryOrUpdate(2, 1n) === 2n && \
               compactTemplate(object) === 'ok' && log === 's';"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn compact_dispatch_coerces_once_and_preserves_thrown_identity() {
    assert_eq!(
        eval(
            "var log = ''; \
             var marker = {}; \
             function compactBinary(add, left, right) { \
               if (add) return left + right; \
               return left - right; \
             } \
             var stringObject = { \
               valueOf: function() { log = log + 'v'; return {}; }, \
               toString: function() { log = log + 's'; return 'x'; } \
             }; \
             var numberObject = { \
               valueOf: function() { log = log + 'n'; return 5; } \
             }; \
             var throwingObject = { \
               valueOf: function() { log = log + 't'; throw marker; } \
             }; \
             var stringValue = compactBinary(true, stringObject, '!'); \
             var numberValue = compactBinary(false, numberObject, 2); \
             var same = false; \
             try { compactBinary(false, throwingObject, 1); } \
             catch (error) { same = error === marker; } \
             stringValue === 'x!' && numberValue === 3 && same && log === 'vsnt';"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn compact_dispatch_preserves_short_circuit_and_tdz() {
    assert_eq!(
        eval(
            "var calls = 0; \
             function tick() { calls = calls + 1; return 'tick'; } \
             function choose(kind, left) { \
               if (kind === 0) return left && tick(); \
               if (kind === 1) return left || tick(); \
               return left ?? tick(); \
             } \
             function readBeforeInit() { return hidden; let hidden = 1; } \
             var andSkip = choose(0, 0); \
             var andTake = choose(0, 1); \
             var orSkip = choose(1, 'kept'); \
             var orTake = choose(1, ''); \
             var nullishSkip = choose(2, 4); \
             var nullishTake = choose(2, null); \
             var sawTdz = false; \
             try { readBeforeInit(); } \
             catch (error) { sawTdz = error instanceof ReferenceError; } \
             andSkip === 0 && andTake === 'tick' && orSkip === 'kept' && \
               orTake === 'tick' && nullishSkip === 4 && \
               nullishTake === 'tick' && calls === 3 && sawTdz;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn compact_dispatch_resumes_zero_to_three_argument_calls_in_order() {
    assert_eq!(
        eval(
            "function zero() { return 'z'; } \
             function one(first) { return first; } \
             function two(first, second) { return first + second; } \
             function three(first, second, third) { \
               return first + second + third; \
             } \
             function dispatch(kind, first, second, third) { \
               if (kind === 0) return zero(); \
               if (kind === 1) return one(first); \
               if (kind === 2) return two(first, second); \
               return three(first, second, third); \
             } \
             dispatch(0) + ':' + dispatch(1, '1') + ':' + \
               dispatch(2, '2', '3') + ':' + dispatch(3, '4', '5', '6');"
        ),
        Ok(Value::String("z:1:23:456".to_owned().into()))
    );
}

#[test]
fn compact_dispatch_keeps_ten_thousand_recursive_frames_off_the_rust_stack() {
    assert_eq!(
        eval(
            "function depth(value) { \
               if (value === 0) return 0; \
               return depth(value - 1) + 1; \
             } \
             depth(10000);"
        ),
        Ok(Value::Number(10_000.0))
    );
}
