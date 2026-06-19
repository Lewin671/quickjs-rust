use crate::{Value, eval};

#[test]
fn evaluates_arithmetic() {
    assert_eq!(eval("1 + 2 * 3;"), Ok(Value::Number(7.0)));
    assert_eq!(eval("0x10 + 0b11 + 0o7;"), Ok(Value::Number(26.0)));
    assert_eq!(eval("0Xf + 0B10 + 0O10;"), Ok(Value::Number(25.0)));
    assert_eq!(eval("1e3 + 1E+2 + 1e-1 + .5e1;"), Ok(Value::Number(1105.1)));
    assert_eq!(
        eval("1_000 + 0x1_0 + 0b10_1 + 0o7_7 + 1_2.3_4 + 1e1_0;"),
        Ok(Value::Number(10_000_001_096.34))
    );
    assert_eq!(eval("true + true;"), Ok(Value::Number(2.0)));
    assert_eq!(eval("true * 2;"), Ok(Value::Number(2.0)));
    assert_eq!(eval("2 ** 3;"), Ok(Value::Number(8.0)));
    assert_eq!(eval("2 ** 3 ** 2;"), Ok(Value::Number(512.0)));
    assert_eq!(eval("3 * 2 ** 3;"), Ok(Value::Number(24.0)));
    assert_eq!(eval("2 ** -1 * 2;"), Ok(Value::Number(1.0)));
}

#[test]
fn evaluates_bitwise_and_shift_expressions() {
    assert_eq!(eval("5 & 3;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("5 | 2;"), Ok(Value::Number(7.0)));
    assert_eq!(eval("5 ^ 3;"), Ok(Value::Number(6.0)));
    assert_eq!(eval("2 << 3;"), Ok(Value::Number(16.0)));
    assert_eq!(eval("-8 >> 1;"), Ok(Value::Number(-4.0)));
    assert_eq!(eval("-1 >>> 0;"), Ok(Value::Number(4_294_967_295.0)));
    assert_eq!(eval("~false;"), Ok(Value::Number(-1.0)));
    assert_eq!(eval("1 + 2 << 3;"), Ok(Value::Number(24.0)));
    assert!(matches!(
        eval("(function() { return 1; }) * {};"),
        Ok(Value::Number(value)) if value.is_nan()
    ));
    assert!(matches!(
        eval("({}) * (function() { return 1; });"),
        Ok(Value::Number(value)) if value.is_nan()
    ));
}

#[test]
fn evaluates_string_addition() {
    assert_eq!(eval("'x' + 1;"), Ok(Value::String("x1".to_owned())));
    assert_eq!(eval("`x` + 1;"), Ok(Value::String("x1".to_owned())));
    assert_eq!(eval("`` + `x`;"), Ok(Value::String("x".to_owned())));
    assert_eq!(
        eval(r#""\x41" + "\u0042" + "\u{43}" + "\A";"#),
        Ok(Value::String("ABCA".to_owned()))
    );
    assert_eq!(eval("\"a\\\nb\";"), Ok(Value::String("ab".to_owned())));
    assert_eq!(eval("1 + 'x';"), Ok(Value::String("1x".to_owned())));
    assert_eq!(eval("'x' + true;"), Ok(Value::String("xtrue".to_owned())));
    assert_eq!(eval("'x' + null;"), Ok(Value::String("xnull".to_owned())));
    assert_eq!(
        eval("'x' + undefined;"),
        Ok(Value::String("xundefined".to_owned()))
    );
    assert_eq!(
        eval("'x' + { valueOf: function() { return 2; } };"),
        Ok(Value::String("x2".to_owned()))
    );
    assert_eq!(
        eval(
            "let object = {}; object[Symbol.toPrimitive] = function(hint) { return hint; }; String(object) + ':' + (+object) + ':' + (object + '');"
        ),
        Ok(Value::String("string:NaN:default".to_owned()))
    );
    assert!(
        eval(
            "let object = {}; object[Symbol.toPrimitive] = function() { return {}; }; object + '';"
        )
        .is_err()
    );
    assert!(eval("let object = {}; object[Symbol.toPrimitive] = 1; object + '';").is_err());
    assert_eq!(
        eval("({ valueOf: function() { return 2; } }) + 3;"),
        Ok(Value::Number(5.0))
    );
    assert_eq!(
        eval("let date = new Date(0); date + 0 === date.toString() + '0';"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let caught = false; try { 'str' + { valueOf: String.prototype.valueOf }; } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn accumulates_string_concatenation_in_place() {
    // The string `+` path appends the right operand onto the left operand's
    // buffer instead of allocating a fresh `len(left) + len(right)` string.
    // Exercise a `s += chunk` accumulation with mixed operand types to confirm
    // the reused buffer still produces the spec-correct result.
    assert_eq!(
        eval("let s = ''; for (let i = 0; i < 5; i++) { s += 'ab'; s += i; s += true; } s;"),
        Ok(Value::String(
            "ab0trueab1trueab2trueab3trueab4true".to_owned()
        ))
    );
    // A non-string left operand promoted by a string right operand must not be
    // corrupted by the in-place append.
    assert_eq!(
        eval("let s = 1; s = s + 'x'; s += 2; s;"),
        Ok(Value::String("1x2".to_owned()))
    );
}

#[test]
fn evaluates_template_literal_substitutions() {
    assert_eq!(
        eval("let name = 'quickjs'; `hello ${name}`;"),
        Ok(Value::String("hello quickjs".to_owned()))
    );
    assert_eq!(
        eval("`${1 + 2}:${true}:${null}:${undefined}`;"),
        Ok(Value::String("3:true:null:undefined".to_owned()))
    );
    assert_eq!(
        eval("`${{ value: 7 }.value}`;"),
        Ok(Value::String("7".to_owned()))
    );
    assert_eq!(
        eval("`escaped \\${name}`;"),
        Ok(Value::String("escaped ${name}".to_owned()))
    );
}

#[test]
fn evaluates_legacy_octal_escapes_inside_template_expressions() {
    assert_eq!(
        eval("`${'\\07'}`;"),
        Ok(Value::String("\u{0007}".to_owned()))
    );
}

#[test]
fn evaluates_annex_b_numeric_literals() {
    assert_eq!(eval("00 + 01 + 07 + 010 + 077;"), Ok(Value::Number(79.0)));
    assert_eq!(eval("08 + 09;"), Ok(Value::Number(17.0)));
    assert!(eval("\"use strict\"; 010;").is_err());
    assert!(eval("\"use strict\"; 08;").is_err());
    assert!(eval("function f() { 'use strict'; return eval('010;'); } f();").is_err());
    assert_eq!(
        eval("function f() { 'use strict'; let indirect = eval; return indirect('010;'); } f();"),
        Ok(Value::Number(8.0))
    );
}

#[test]
fn template_literal_substitutions_to_string_before_next_expression() {
    assert_eq!(
        eval(
            "let log = ''; let first = { toString: function() { log += 'a'; return 'A'; } }; let second = { toString: function() { log += 'b'; return 'B'; } }; `${first}${second}` + ':' + log;"
        ),
        Ok(Value::String("AB:ab".to_owned()))
    );
}

#[test]
fn evaluates_tagged_template_literals() {
    assert_eq!(
        eval(
            "function tag(strings, a, b) { return strings[0] + ':' + strings[1] + ':' + strings.raw[1] + ':' + a + ':' + b; } tag`a ${1 + 1} \\n${3} b`;"
        ),
        Ok(Value::String("a : \n: \\n:2:3".to_owned()))
    );
    assert_eq!(
        eval(
            "let object = { prefix: 'p', tag: function(strings, value) { return this.prefix + ':' + strings[0] + ':' + strings.raw[0] + ':' + value; } }; object.tag`\\t${4}`;"
        ),
        Ok(Value::String("p:\t:\\t:4".to_owned()))
    );
    assert_eq!(
        eval("function tag(strings) { return strings[0] + ':' + strings.raw[0]; } tag`plain`;"),
        Ok(Value::String("plain:plain".to_owned()))
    );
}

#[test]
fn evaluates_comparison_and_equality() {
    assert_eq!(eval("1 + 2 * 3 >= 7;"), Ok(Value::Boolean(true)));
    assert_eq!(eval("'2' < '10';"), Ok(Value::Boolean(false)));
    assert_eq!(eval("'\\u{10000}' <= '\\uFFFF';"), Ok(Value::Boolean(true)));
    assert_eq!(
        eval("'\\u{10000}' >= '\\uFFFF';"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval(
            "let object = { valueOf: function() { return -2; }, toString: function() { return '-2'; } }; '-1' < object;"
        ),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval(
            "let object = { valueOf: function() { return '-2'; }, toString: function() { return -2; } }; object < '-1';"
        ),
        Ok(Value::Boolean(false))
    );
    assert_eq!(eval("1 + 1 === 2;"), Ok(Value::Boolean(true)));
    assert_eq!(eval("1 !== 2;"), Ok(Value::Boolean(true)));
    assert_eq!(eval("null == undefined;"), Ok(Value::Boolean(true)));
    assert_eq!(eval("null != undefined;"), Ok(Value::Boolean(false)));
    assert_eq!(eval("'1' == 1;"), Ok(Value::Boolean(true)));
    assert_eq!(eval("1 == '1';"), Ok(Value::Boolean(true)));
    assert_eq!(eval("true == 1;"), Ok(Value::Boolean(true)));
    assert_eq!(eval("false == 0;"), Ok(Value::Boolean(true)));
    assert_eq!(eval("false == '';"), Ok(Value::Boolean(true)));
    assert_eq!(eval("new Boolean(true) == true;"), Ok(Value::Boolean(true)));
    assert_eq!(eval("true == new String('+1');"), Ok(Value::Boolean(true)));
    assert_eq!(
        eval("({ valueOf: function() { return 1; } }) == true;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("'+1' == { valueOf: function() { return 1; }, toString: function() { return {}; } };"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(eval("NaN == NaN;"), Ok(Value::Boolean(false)));
    assert_eq!(eval("'x' == 1;"), Ok(Value::Boolean(false)));
    assert_eq!(eval("'1' === 1;"), Ok(Value::Boolean(false)));
    assert_eq!(
        eval("function C() {} let instance = new C(); instance instanceof C;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("function C() {} function D() {} let instance = new C(); instance instanceof D;"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval("function C() {} 1 instanceof C;"),
        Ok(Value::Boolean(false))
    );
    assert!(eval("let object = {}; object instanceof {};").is_err());
    assert!(
        eval("function C() {} C.prototype = 1; let object = {}; object instanceof C;").is_err()
    );
    // A Symbol-valued `prototype` is not an object: OrdinaryHasInstance throws.
    assert!(
        eval("function C() {} C.prototype = Symbol(); let object = {}; object instanceof C;")
            .is_err()
    );
    // A Symbol primitive operand is not an object, so it is never an instance.
    assert_eq!(
        eval("function C() {} Symbol() instanceof C;"),
        Ok(Value::Boolean(false))
    );
    // ToObject boxes a Symbol receiver: Array.prototype.sort returns the
    // Symbol wrapper object, which is an instance of Symbol.
    assert_eq!(
        eval("[].sort.call(Symbol()) instanceof Symbol;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("typeof [].sort.call(Symbol());"),
        Ok(Value::String("object".to_owned()))
    );
    assert_eq!(
        eval(
            "let calls = 0; let F = {}; F[Symbol.hasInstance] = function(value) { calls = calls + (this === F ? 1 : 0); return value === 7; }; (7 instanceof F) + ':' + (8 instanceof F) + ':' + calls;"
        ),
        Ok(Value::String("true:false:2".to_owned()))
    );
    assert_eq!(
        eval(
            "let F = {}; F[Symbol.hasInstance] = function(value) { return value === 1 ? 'yes' : ''; }; (1 instanceof F) + ':' + (2 instanceof F);"
        ),
        Ok(Value::String("true:false".to_owned()))
    );
    assert_eq!(
        eval(
            "function F() {} Object.defineProperty(F, Symbol.hasInstance, { value: function(value) { return value === 3; }, configurable: true }); (3 instanceof F) + ':' + (4 instanceof F);"
        ),
        Ok(Value::String("true:false".to_owned()))
    );
    assert_eq!(
        eval(
            "function F() {} F[Symbol.hasInstance] = function() { return true; }; let object = {}; object instanceof F;"
        ),
        Ok(Value::Boolean(false))
    );
    assert!(
        eval("let F = {}; F[Symbol.hasInstance] = 1; let caught = false; try { 1 instanceof F; } catch (error) { caught = error instanceof TypeError; } caught;")
            .is_ok_and(|value| value == Value::Boolean(true))
    );
}

#[test]
fn evaluates_logical_expressions() {
    assert_eq!(eval("0 || 5;"), Ok(Value::Number(5.0)));
    assert_eq!(eval("1 && 7;"), Ok(Value::Number(7.0)));
}

#[test]
fn evaluates_nullish_coalescing_expressions() {
    assert_eq!(eval("null ?? 42;"), Ok(Value::Number(42.0)));
    assert_eq!(eval("undefined ?? 42;"), Ok(Value::Number(42.0)));
    assert_eq!(eval("0 ?? 42;"), Ok(Value::Number(0.0)));
    assert_eq!(eval("false ?? 42;"), Ok(Value::Boolean(false)));
    assert_eq!(eval("42 ?? missing;"), Ok(Value::Number(42.0)));
    assert_eq!(eval("null ?? 0 ?? 1;"), Ok(Value::Number(0.0)));
}

#[test]
fn evaluates_conditional_expressions() {
    assert_eq!(eval("true ? 1 : 2;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("false ? 1 : 2;"), Ok(Value::Number(2.0)));
    assert_eq!(
        eval("let x = true ? 'yes' : 'no'; x;"),
        Ok(Value::String("yes".to_owned()))
    );
    assert_eq!(eval("true ? 1 : missing;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("false ? missing : 2;"), Ok(Value::Number(2.0)));
}

#[test]
fn evaluates_sequence_expressions() {
    assert_eq!(eval("1, 2;"), Ok(Value::Number(2.0)));
    assert_eq!(
        eval("let x = 0; x = 1, x = x + 2, x;"),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval("let x = 0; while ((x = x + 1, x < 3)) { } x;"),
        Ok(Value::Number(3.0))
    );
}

#[test]
fn evaluates_assignment_expressions() {
    assert_eq!(eval("let x = 2; x = x + 3; x;"), Ok(Value::Number(5.0)));
}

#[test]
fn evaluates_update_and_compound_assignment() {
    assert_eq!(eval("let x = 1; x++; x;"), Ok(Value::Number(2.0)));
    assert_eq!(eval("let x = 1; ++x;"), Ok(Value::Number(2.0)));
    assert_eq!(eval("let x = 1; x++;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("let x = false; x++;"), Ok(Value::Number(0.0)));
    assert_eq!(eval("let x = 3; x--; x;"), Ok(Value::Number(2.0)));
    assert_eq!(eval("let x = 1; x += 2; x;"), Ok(Value::Number(3.0)));
    assert_eq!(eval("let x = -3; x **= 3; x;"), Ok(Value::Number(-27.0)));
    assert_eq!(eval("let x = 2; x <<= 3; x;"), Ok(Value::Number(16.0)));
    assert_eq!(eval("let x = -8; x >>= 1; x;"), Ok(Value::Number(-4.0)));
    assert_eq!(
        eval("let x = -1; x >>>= 0; x;"),
        Ok(Value::Number(4_294_967_295.0))
    );
    assert_eq!(eval("let x = 5; x &= 3; x;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("let x = 5; x ^= 3; x;"), Ok(Value::Number(6.0)));
    assert_eq!(eval("let x = 5; x |= 2; x;"), Ok(Value::Number(7.0)));
    assert_eq!(
        eval("let x = 'a'; x += 1; x;"),
        Ok(Value::String("a1".to_owned()))
    );
    assert_eq!(
        eval("let o = { count: 1 }; o.count++; o.count;"),
        Ok(Value::Number(2.0))
    );
}

#[test]
fn evaluates_logical_assignment() {
    assert_eq!(eval("let x = 0; x &&= missing; x;"), Ok(Value::Number(0.0)));
    assert_eq!(eval("let x = 2; x &&= 7; x;"), Ok(Value::Number(7.0)));
    assert_eq!(eval("let x = 0; x ||= 7; x;"), Ok(Value::Number(7.0)));
    assert_eq!(eval("let x = 2; x ||= missing; x;"), Ok(Value::Number(2.0)));
    assert_eq!(eval("let x = null; x ??= 7; x;"), Ok(Value::Number(7.0)));
    assert_eq!(
        eval("let x = undefined; x ??= 8; x;"),
        Ok(Value::Number(8.0))
    );
    assert_eq!(
        eval("let x = false; x ??= missing; x;"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval("let o = { value: 0 }; o.value ||= 3; o.value;"),
        Ok(Value::Number(3.0))
    );
}

#[test]
fn evaluates_unary_expressions() {
    assert_eq!(eval("-1 + 3;"), Ok(Value::Number(2.0)));
    assert_eq!(eval("!0;"), Ok(Value::Boolean(true)));
    assert_eq!(eval("+true;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("+'Infinity';"), Ok(Value::Number(f64::INFINITY)));
    assert!(matches!(eval("+'INFINITY';"), Ok(Value::Number(value)) if value.is_nan()));
    assert!(matches!(eval("+'infinity';"), Ok(Value::Number(value)) if value.is_nan()));
    assert_eq!(eval("void 0;"), Ok(Value::Undefined));
    assert_eq!(eval("let x = 0; void (x = 1); x;"), Ok(Value::Number(1.0)));
}

#[test]
fn evaluates_typeof_expressions() {
    assert_eq!(
        eval("typeof undefined;"),
        Ok(Value::String("undefined".to_owned()))
    );
    assert_eq!(
        eval("typeof neverDeclared;"),
        Ok(Value::String("undefined".to_owned()))
    );
    assert_eq!(
        eval("typeof true;"),
        Ok(Value::String("boolean".to_owned()))
    );
    assert_eq!(eval("typeof 1;"), Ok(Value::String("number".to_owned())));
    assert_eq!(eval("typeof 'x';"), Ok(Value::String("string".to_owned())));
    assert_eq!(eval("typeof null;"), Ok(Value::String("object".to_owned())));
    assert_eq!(eval("typeof {};"), Ok(Value::String("object".to_owned())));
    assert_eq!(eval("typeof this;"), Ok(Value::String("object".to_owned())));
    assert_eq!(
        eval("function f() { return 1; } typeof f;"),
        Ok(Value::String("function".to_owned()))
    );
}

#[test]
fn evaluates_delete_operator() {
    assert_eq!(eval("let o = {}; delete o.x;"), Ok(Value::Boolean(true)));
    assert_eq!(
        eval("let o = { red: 1 }; delete o.red; o.red;"),
        Ok(Value::Undefined)
    );
    assert_eq!(
        eval("let o = { 2: 2 }; delete o[2]; o['2'];"),
        Ok(Value::Undefined)
    );
    assert_eq!(
        eval("let key = Symbol(); let o = { [key]: 2 }; delete o[key] && o[key] === undefined;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("let o = {}; Object.defineProperty(o, 'fixed', { value: 1 }); delete o.fixed;"),
        Ok(Value::Boolean(false))
    );
}

#[test]
fn evaluates_in_operator() {
    assert_eq!(
        eval("'answer' in { answer: 42 };"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("'missing' in { answer: 42 };"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval("let object = {}; object.Infinity = 1; Infinity in object;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("let symbol = Symbol(); let object = { [symbol]: 1 }; symbol in object;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let symbol = Symbol(); let other = Symbol(); let object = { [symbol]: 1 }; other in object;"
        ),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval("let o = {}; o.present = undefined; 'present' in o;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(eval("'length' in [1, 2];"), Ok(Value::Boolean(true)));
    assert_eq!(eval("'call' in function f() {};"), Ok(Value::Boolean(true)));
    assert_eq!(
        eval(
            "let proto = { marker: 1 }; let array = []; Object.setPrototypeOf(array, proto); 'marker' in array;"
        ),
        Ok(Value::Boolean(true))
    );
    assert!(eval("'a' in 1;").is_err());
}

#[test]
fn evaluates_destructuring_assignment_expressions() {
    assert_eq!(
        eval("var a = 1, b = 2; [a, b] = [b, a]; a + ':' + b;"),
        Ok(Value::String("2:1".to_owned()))
    );
    assert_eq!(eval("var y; ({y = 9} = {}); y;"), Ok(Value::Number(9.0)));
    assert_eq!(
        eval(
            "var out = {}; var y, rest; ({x: out.first, y, ...rest} = {x: 1, y: 2, z: 3});
             out.first + ':' + y + ':' + Object.keys(rest).join('|');"
        ),
        Ok(Value::String("1:2:z".to_owned()))
    );
    assert_eq!(
        eval("var x, y; [[x], {p: y = 7}] = [[5], {}]; x + y;"),
        Ok(Value::Number(12.0))
    );
    assert_eq!(
        eval(
            "var source = {p: 42}; var p; var result = ({p} = source); (result === source) + ':' + p;"
        ),
        Ok(Value::String("true:42".to_owned()))
    );
    assert_eq!(
        eval(
            "var rest; var calls = []; var s = Symbol('foo');
             var o = { get z() { calls.push('z'); }, get a() { calls.push('a'); } };
             Object.defineProperty(o, 1, { get: function() { calls.push(1); }, enumerable: true });
             Object.defineProperty(o, s, { get: function() { calls.push('Symbol(foo)'); }, enumerable: true });
             var result = ({...rest} = o);
             calls.join('|') + ':' + Object.keys(rest).join('|') + ':' + (Object.getOwnPropertySymbols(rest)[0] === s) + ':' + (result === o);"
        ),
        Ok(Value::String("1|z|a|Symbol(foo):1|z|a:true:true".to_owned()))
    );
}

#[test]
fn assignment_in_with_uses_initial_reference() {
    assert_eq!(
        eval(
            "function testFunction() {
               var x = 0;
               var scope = {x: 1};
               with (scope) {
                 x = (delete scope.x, 2);
               }
               return scope.x + ':' + x;
             }
             testFunction();"
        ),
        Ok(Value::String("2:0".to_owned()))
    );
    assert_eq!(
        eval(
            "var x = 0;
             var scope = {x: 1};
             with (scope) {
               x = (delete scope.x, 2);
             }
             scope.x + ':' + x;"
        ),
        Ok(Value::String("2:0".to_owned()))
    );
    assert_eq!(
        eval(
            "var outerScope = {x: 0};
             var innerScope = {x: 1};
             with (outerScope) {
               with (innerScope) {
                 x = (delete innerScope.x, 2);
               }
             }
             innerScope.x + ':' + outerScope.x;"
        ),
        Ok(Value::String("2:0".to_owned()))
    );
    assert_eq!(
        eval(
            "function testAssignment() {
               var x = 0;
               var scope = {};
               with (scope) {
                 x = (scope.x = 2, 1);
               }
               return scope.x + ':' + x;
             }
             testAssignment();"
        ),
        Ok(Value::String("2:1".to_owned()))
    );
}

#[test]
fn strict_compound_assignment_rechecks_resolved_object_environment_binding() {
    assert_eq!(
        eval(
            "var count = 0;
             var scope = { get x() { delete this.x; return 2; } };
             with (scope) {
               (function() {
                 'use strict';
                 try { count++; x += 1; count++; }
                 catch (error) { count += error instanceof ReferenceError ? 1 : 100; }
               })();
             }
             count + ':' + ('x' in scope);"
        ),
        Ok(Value::String("2:false".to_owned()))
    );
    assert_eq!(
        eval(
            "var count = 0;
             Object.defineProperty(this, 'x', {
               configurable: true,
               get: function() { delete this.x; return 2; }
             });
             (function() {
               'use strict';
               try { count++; x ^= 3; count++; }
               catch (error) { count += error instanceof ReferenceError ? 1 : 100; }
             })();
             count + ':' + ('x' in this);"
        ),
        Ok(Value::String("2:false".to_owned()))
    );
}

#[test]
fn member_compound_assignment_evaluates_reference_before_rhs() {
    assert_eq!(
        eval(
            "var hits = '';
             var prop = function() { hits += 'prop'; throw 'key'; };
             var rhs = function() { hits += ':rhs'; return 1; };
             try { var base = null; base[prop()] *= rhs(); }
             catch (error) { hits += ':' + error; }
             hits;"
        ),
        Ok(Value::String("prop:key".to_owned()))
    );
    assert_eq!(
        eval(
            "var hits = '';
             var prop = { toString: function() { hits += ':toString'; return 'x'; } };
             var rhs = function() { hits += ':rhs'; return 1; };
             try { var base = null; base[prop] *= rhs(); }
             catch (error) { hits += error instanceof TypeError ? ':type' : ':other'; }
             hits;"
        ),
        Ok(Value::String(":type".to_owned()))
    );
    assert_eq!(
        eval(
            "var hits = 0;
             var base = {};
             var prop = { toString: function() { hits++; return 'x'; } };
             base[prop] *= 2;
             hits;"
        ),
        Ok(Value::Number(1.0))
    );
}

#[test]
fn member_assignment_delays_put_value_checks_until_after_rhs() {
    assert_eq!(
        eval(
            "var hits = '';
             var prop = function() { hits += 'prop'; throw 'key'; };
             var rhs = function() { hits += ':rhs'; return 1; };
             try { var base = null; base[prop()] = rhs(); }
             catch (error) { hits += ':' + error; }
             hits;"
        ),
        Ok(Value::String("prop:key".to_owned()))
    );
    assert_eq!(
        eval(
            "var hits = '';
             var prop = { toString: function() { hits += ':toString'; throw 'key'; } };
             var rhs = function() { hits += ':rhs'; throw 'rhs'; };
             try { var base = null; base[prop] = rhs(); }
             catch (error) { hits += ':' + error; }
             hits;"
        ),
        Ok(Value::String(":rhs:rhs".to_owned()))
    );
    assert_eq!(
        eval(
            "var hits = 0;
             try { var base = null; base.x = (hits += 1); }
             catch (error) { hits += error instanceof TypeError ? 10 : 100; }
             hits;"
        ),
        Ok(Value::Number(11.0))
    );
    assert_eq!(
        eval(
            "var hits = 0;
             try { var base = undefined; base.x = (hits += 1); }
             catch (error) { hits += error instanceof TypeError ? 10 : 100; }
             hits;"
        ),
        Ok(Value::Number(11.0))
    );
}

#[test]
fn assignments_respect_lexical_tdz() {
    assert_eq!(
        eval(
            "var result = '';
             try { x = 1; result = 'no throw'; }
             catch (error) { result = error instanceof ReferenceError ? 'reference' : error.name; }
             let x;
             result;"
        ),
        Ok(Value::String("reference".to_owned()))
    );
    assert_eq!(
        eval(
            "var result = '';
             try { 0, [x] = []; result = 'no throw'; }
             catch (error) { result = error instanceof ReferenceError ? 'reference' : error.name; }
             let x;
             result;"
        ),
        Ok(Value::String("reference".to_owned()))
    );
    assert_eq!(
        eval(
            "var result = '';
             var x;
             try { 0, [x = y] = []; result = 'no throw'; }
             catch (error) { result = error instanceof ReferenceError ? 'reference' : error.name; }
             let y;
             result;"
        ),
        Ok(Value::String("reference".to_owned()))
    );
}

#[test]
fn nested_destructuring_assignments_capture_later_script_lexicals_tdz() {
    assert_eq!(
        eval(
            "var result = '';
             try { (function() { 0, [x] = []; })(); result = 'no throw'; }
             catch (error) { result = error instanceof ReferenceError ? 'reference' : error.name; }
             let x;
             result;"
        ),
        Ok(Value::String("reference".to_owned()))
    );
    assert_eq!(
        eval(
            "var x;
             var result = '';
             try { (function() { 0, [x = y] = []; })(); result = 'no throw'; }
             catch (error) { result = error instanceof ReferenceError ? 'reference' : error.name; }
             let y;
             result;"
        ),
        Ok(Value::String("reference".to_owned()))
    );
    assert_eq!(
        eval(
            "var result = '';
             try { (function() { 0, [...x] = []; })(); result = 'no throw'; }
             catch (error) { result = error instanceof ReferenceError ? 'reference' : error.name; }
             let x;
             result;"
        ),
        Ok(Value::String("reference".to_owned()))
    );
}

#[test]
fn typeof_respects_lexical_tdz() {
    assert_eq!(
        eval(
            "var result = '';
             try { typeof x; result = 'no throw'; }
             catch (error) { result = error instanceof ReferenceError ? 'reference' : error.name; }
             let x;
             result;"
        ),
        Ok(Value::String("reference".to_owned()))
    );
    assert_eq!(
        eval("typeof definitelyUnresolvable;"),
        Ok(Value::String("undefined".to_owned()))
    );
}

#[test]
fn destructuring_assignment_evaluates_member_targets_before_value_reads() {
    assert_eq!(
        eval(
            "var order = []; var out = {};
             var target = function() { order.push('lref'); return out; };
             var source = {};
             Object.defineProperty(source, 'a', { get: function() { order.push('get'); return 1; } });
             ({a: target().x} = source);
             order.join(',') + '|' + out.x;"
        ),
        Ok(Value::String("lref,get|1".to_owned()))
    );
}

#[test]
fn destructuring_assignment_evaluates_computed_keys_before_member_targets() {
    assert_eq!(
        eval(
            "var order = [];
             var key = { toString: function() { order.push('key'); return 'a'; } };
             var target = function() {
               order.push('target');
               return { set x(value) { order.push('set:' + value); } };
             };
             var source = {};
             Object.defineProperty(source, 'a', { get: function() { order.push('get'); return 7; } });
             ({[key]: target().x} = source);
             order.join(',');"
        ),
        Ok(Value::String("key,target,get,set:7".to_owned()))
    );
}

#[test]
fn destructuring_assignment_closes_iterators() {
    assert_eq!(
        eval(
            "var returned = false; var iterable = {};
             iterable[Symbol.iterator] = function() {
               return {
                 next: function() { return { value: 1, done: false }; },
                 return: function() { returned = true; return {}; }
               };
             };
             var a; [a] = iterable; returned + ':' + a;"
        ),
        Ok(Value::String("true:1".to_owned()))
    );
    assert_eq!(
        eval(
            "var nextCount = 0; var returnCount = 0; var iterable = {};
             var iterator = {
               next: function() { nextCount += 1; return { done: true }; },
               return: function() { returnCount += 1; return {}; }
             };
             iterable[Symbol.iterator] = function() { return iterator; };
             var thrower = function() { throw new Error('key'); };
             var caught = false;
             try { 0, [ ({})[thrower()] ] = iterable; } catch (error) { caught = true; }
             caught + ':' + nextCount + ':' + returnCount;"
        ),
        Ok(Value::String("true:0:1".to_owned()))
    );
    assert_eq!(
        eval(
            "var returnCount = 0; var unreachable = 0; var iterator = {
               next: function() { return { done: false, value: undefined }; },
               return: function() { returnCount += 1; return {}; }
             };
             var iterable = {};
             iterable[Symbol.iterator] = function() { return iterator; };
             function* g() {
               var target;
               var vals = iterable;
               0, [target = yield] = vals;
               unreachable += 1;
             }
             var iter = g();
             iter.next();
             var result = iter.return(777);
             returnCount + ':' + unreachable + ':' + result.value + ':' + result.done;"
        ),
        Ok(Value::String("1:0:777:true".to_owned()))
    );
    assert_eq!(
        eval(
            "var returnCount = 0; var iterator = {
               next: function() { return { done: false, value: undefined }; },
               return: function() { returnCount += 1; throw new Error('close'); }
             };
             var iterable = {};
             iterable[Symbol.iterator] = function() { return iterator; };
             function* g() { var target; 0, [target = yield] = iterable; }
             var iter = g();
             iter.next();
             var caught = '';
             try { iter.return(777); } catch (error) { caught = error.message; }
             caught + ':' + returnCount;"
        ),
        Ok(Value::String("close:1".to_owned()))
    );
    assert_eq!(
        eval(
            "var returnCount = 0; var iterator = {
               next: function() { return { done: false, value: undefined }; },
               return: function() { returnCount += 1; return null; }
             };
             var iterable = {};
             iterable[Symbol.iterator] = function() { return iterator; };
             function* g() { var target; 0, [target = yield] = iterable; }
             var iter = g();
             iter.next();
             var caught = '';
             try { iter.return(777); } catch (error) { caught = error instanceof TypeError ? 'type' : error.name; }
             caught + ':' + returnCount;"
        ),
        Ok(Value::String("type:1".to_owned()))
    );
}

#[test]
fn optional_chaining_method_call_keeps_receiver_this() {
    // `obj.m?.()`, `obj?.m()`, and computed forms must call `m` with `this`
    // bound to `obj`, matching a normal method call.
    assert_eq!(
        eval("var o = { x: 7, m() { return this.x; } }; o.m?.();"),
        Ok(Value::Number(7.0))
    );
    assert_eq!(
        eval("var o = { x: 7, m() { return this.x; } }; o?.m();"),
        Ok(Value::Number(7.0))
    );
    assert_eq!(
        eval("var o = { x: 7, m() { return this.x; } }; o?.m?.();"),
        Ok(Value::Number(7.0))
    );
    assert_eq!(
        eval("var o = { x: 7, m() { return this.x; } }; o['m']?.();"),
        Ok(Value::Number(7.0))
    );
    // A nullish base short-circuits the whole call to undefined without
    // evaluating arguments or throwing.
    assert_eq!(
        eval("var n = null; var ran = false; var r = n?.m(ran = true); r + ':' + ran;"),
        Ok(Value::String("undefined:false".to_owned()))
    );
    // The receiver is evaluated exactly once.
    assert_eq!(
        eval(
            "var calls = 0;
             function get() { calls++; return { m() { return 1; } }; }
             get().m?.();
             calls;"
        ),
        Ok(Value::Number(1.0))
    );
}

#[test]
fn optional_chaining_short_circuits_through_calls() {
    // A nullish link short-circuits the entire chain, including a trailing
    // member after a call, without evaluating the call or its arguments.
    assert_eq!(
        eval("var a = undefined; var x = 1; a?.b.c(++x).d; x;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("var a = undefined; var x = 1; a?.[++x]; x;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("var o = null; var r = o?.m().n; typeof r;"),
        Ok(Value::String("undefined".to_owned()))
    );
    // A live chain still threads results and receivers correctly.
    assert_eq!(
        eval("var a = { b: { c(v) { return { d: v * 10 }; } } }; a?.b.c(5).d;"),
        Ok(Value::Number(50.0))
    );
}

#[test]
fn optional_chaining_on_new_target_and_super() {
    // `new.target` is a MetaProperty and may head an optional chain.
    assert_eq!(
        eval("function f() { return new.target?.name; } f();"),
        Ok(Value::Undefined)
    );
    assert_eq!(
        eval("function f() { return new.target?.name; } new f().wrapped ? 'x' : (new f(), 'ran');"),
        Ok(Value::String("ran".to_owned()))
    );
    // `super.x` heads an optional chain through the super-property path.
    assert_eq!(
        eval(
            "class B { get p() { return { q: 9 }; } }
             class A extends B { m() { return super.p?.q; } }
             new A().m();"
        ),
        Ok(Value::Number(9.0))
    );
    assert_eq!(
        eval(
            "class B {}
             class A extends B { m() { return super.zzz?.q; } }
             new A().m();"
        ),
        Ok(Value::Undefined)
    );
    assert_eq!(
        eval(
            "class B { a() { return { c: 5 }; } }
             class A extends B { m() { return super.a?.().c; } }
             new A().m();"
        ),
        Ok(Value::Number(5.0))
    );
}
