use crate::{Value, eval};

#[test]
fn evaluates_math_builtins() {
    assert_eq!(eval("typeof Math;"), Ok(Value::String("object".to_owned())));
    assert_eq!(
        eval("Object.prototype.toString.call(Math);"),
        Ok(Value::String("[object Math]".to_owned()))
    );
    assert_eq!(
        eval("typeof Math.PI;"),
        Ok(Value::String("number".to_owned()))
    );
    assert_eq!(eval("NaN === NaN;"), Ok(Value::Boolean(false)));
    assert_eq!(eval("Infinity === 1 / 0;"), Ok(Value::Boolean(true)));
    assert_eq!(eval("Math.abs.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Math.acos.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Math.acosh.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Math.asin.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Math.asinh.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Math.atan.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Math.atan2.length;"), Ok(Value::Number(2.0)));
    assert_eq!(eval("Math.atanh.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Math.cbrt.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Math.cos.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Math.cosh.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Math.exp.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Math.expm1.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Math.f16round.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Math.fround.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Math.hypot.length;"), Ok(Value::Number(2.0)));
    assert_eq!(eval("Math.log.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Math.log1p.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Math.log10.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Math.log2.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Math.max.length;"), Ok(Value::Number(2.0)));
    assert_eq!(eval("Math.min.length;"), Ok(Value::Number(2.0)));
    assert_eq!(eval("Math.pow.length;"), Ok(Value::Number(2.0)));
    assert_eq!(eval("Math.random.length;"), Ok(Value::Number(0.0)));
    assert_eq!(eval("Math.sqrt.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Math.sumPrecise.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Math.round.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Math.sign.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Math.sin.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Math.sinh.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Math.clz32.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Math.imul.length;"), Ok(Value::Number(2.0)));
    assert_eq!(eval("Math.tan.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Math.tanh.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Math.trunc.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Math.abs(-7);"), Ok(Value::Number(7.0)));
    assert_eq!(
        eval("1 / Math.abs(-0) === Infinity;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(eval("Math.ceil(1.2);"), Ok(Value::Number(2.0)));
    assert_eq!(eval("Math.floor(1.8);"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Math.trunc(-1.8);"), Ok(Value::Number(-1.0)));
    assert_eq!(eval("Math.max(1, 9, 3);"), Ok(Value::Number(9.0)));
    assert_eq!(eval("Math.max() === -Infinity;"), Ok(Value::Boolean(true)));
    assert_eq!(eval("Math.min(1, -9, 3);"), Ok(Value::Number(-9.0)));
    assert_eq!(eval("Math.min() === Infinity;"), Ok(Value::Boolean(true)));
    assert_eq!(
        eval("1 / Math.max(-0, 0) === Infinity;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("1 / Math.min(-0, 0) === -Infinity;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Math.max(1, NaN) === Math.max(1, NaN);"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval("Math.min(1, NaN) === Math.min(1, NaN);"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(eval("Math.pow(2, 8);"), Ok(Value::Number(256.0)));
    assert_eq!(
        eval(
            "let random = Math.random(); typeof random === 'number' && random >= 0 && random < 1;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Math.pow(2, NaN) === Math.pow(2, NaN);"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval("Math.pow(1, NaN) === Math.pow(1, NaN);"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(eval("Math.sqrt(81);"), Ok(Value::Number(9.0)));
    assert_eq!(
        eval("Math.sqrt(-1) === Math.sqrt(-1);"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(eval("Math.round(1.5);"), Ok(Value::Number(2.0)));
    assert_eq!(eval("Math.round(-1.5);"), Ok(Value::Number(-1.0)));
    assert_eq!(
        eval("1 / Math.round(-0.4) === -Infinity;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(eval("Math.sign(-7);"), Ok(Value::Number(-1.0)));
    assert_eq!(eval("Math.sign(7);"), Ok(Value::Number(1.0)));
    assert_eq!(
        eval("1 / Math.sign(-0) === -Infinity;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Math.sign(NaN) === Math.sign(NaN);"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(eval("Math.clz32(0);"), Ok(Value::Number(32.0)));
    assert_eq!(eval("Math.clz32(1);"), Ok(Value::Number(31.0)));
    assert_eq!(eval("Math.clz32(4294967295);"), Ok(Value::Number(0.0)));
    assert_eq!(eval("Math.imul(2, 4);"), Ok(Value::Number(8.0)));
    assert_eq!(eval("Math.imul(-1, 8);"), Ok(Value::Number(-8.0)));
    assert_eq!(eval("Math.imul(4294967295, 5);"), Ok(Value::Number(-5.0)));
    assert_eq!(eval("Math.sin(0);"), Ok(Value::Number(0.0)));
    assert_eq!(eval("Math.cos(0);"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Math.tan(0);"), Ok(Value::Number(0.0)));
    assert_eq!(eval("Math.asin(0);"), Ok(Value::Number(0.0)));
    assert_eq!(eval("Math.atan(0);"), Ok(Value::Number(0.0)));
    assert_eq!(eval("Math.atan2(0, 1);"), Ok(Value::Number(0.0)));
    assert_eq!(eval("Math.cbrt(27);"), Ok(Value::Number(3.0)));
    assert_eq!(eval("Math.exp(0);"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Math.log(1);"), Ok(Value::Number(0.0)));
    assert_eq!(eval("Math.log10(1000);"), Ok(Value::Number(3.0)));
    assert_eq!(eval("Math.log2(8);"), Ok(Value::Number(3.0)));
    assert_eq!(
        eval("Math.acos(NaN) === Math.acos(NaN);"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval("Math.log(-1) === Math.log(-1);"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval("Math.log10(0) === -Infinity;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Math.log2(0) === -Infinity;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(eval("Math.acosh(1);"), Ok(Value::Number(0.0)));
    assert_eq!(eval("Math.asinh(0);"), Ok(Value::Number(0.0)));
    assert_eq!(eval("Math.atanh(0);"), Ok(Value::Number(0.0)));
    assert_eq!(
        eval("Math.atanh(1) === Infinity;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(eval("Math.cosh(0);"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Math.expm1(0);"), Ok(Value::Number(0.0)));
    assert_eq!(
        eval("Math.f16round(1.00048828125);"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("Math.f16round(1.0009765625);"),
        Ok(Value::Number(1.0009765625))
    );
    assert_eq!(eval("Math.f16round(65519);"), Ok(Value::Number(65504.0)));
    assert_eq!(
        eval("Math.f16round(65520) === Infinity;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("1 / Math.f16round(-0) === -Infinity;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Math.f16round(NaN) === Math.f16round(NaN);"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(eval("Math.fround(1.5);"), Ok(Value::Number(1.5)));
    assert_eq!(
        eval("1 / Math.fround(-0) === -Infinity;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(eval("Math.hypot(3, 4);"), Ok(Value::Number(5.0)));
    assert_eq!(eval("Math.hypot();"), Ok(Value::Number(0.0)));
    assert_eq!(
        eval("Math.hypot(Infinity, NaN) === Infinity;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(eval("Math.log1p(0);"), Ok(Value::Number(0.0)));
    assert_eq!(eval("Math.sumPrecise([1, 2, 3]);"), Ok(Value::Number(6.0)));
    assert_eq!(
        eval("Math.sumPrecise([1e30, 0.1, -1e30]);"),
        Ok(Value::Number(0.1))
    );
    assert_eq!(
        eval("1 / Math.sumPrecise([]) === -Infinity;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Math.sumPrecise([Infinity, -Infinity]) === Math.sumPrecise([Infinity, -Infinity]);"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval(
            "let caught = false; try { Math.sumPrecise([1, '2']); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("function* values() { yield 1; yield 2; } Math.sumPrecise(values());"),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval(
            "let values = [4]; values[Symbol.iterator] = function* () { yield 1; yield 2; }; Math.sumPrecise(values);"
        ),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval(
            "let closed = 0; let iterator = { next() { return { done: false, value: {} }; }, return() { closed += 1; return {}; } }; let iterable = { [Symbol.iterator]() { return iterator; } }; let caught = false; try { Math.sumPrecise(iterable); } catch (error) { caught = error instanceof TypeError; } caught && closed === 1;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Math.log1p(-1) === -Infinity;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(eval("Math.sinh(0);"), Ok(Value::Number(0.0)));
    assert_eq!(eval("Math.tanh(Infinity);"), Ok(Value::Number(1.0)));
    assert_eq!(
        eval("1 / Math.tanh(-0) === -Infinity;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Math.propertyIsEnumerable('PI');"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval("Object.getOwnPropertyDescriptor(Math, 'PI').writable;"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval(
            "let d = Object.getOwnPropertyDescriptor(Math, 'random'); d.enumerable + ':' + d.writable + ':' + d.configurable;"
        ),
        Ok(Value::String("false:true:true".to_owned()))
    );
    assert!(eval("new Math.abs(1);").is_err());
}
