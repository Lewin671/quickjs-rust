// Derived from: test/built-ins/Number/S15.7.1.1_A1.js
if (typeof Number("10") !== "number") {
  throw "expected Number('10') to produce a number";
}
if (Number() !== 0) {
  throw "expected Number() to return 0";
}
if (Number(undefined) === Number(undefined)) {
  throw "expected Number(undefined) to return NaN";
}
if (Number("10") !== 10) {
  throw "expected Number('10') to return 10";
}
if (Number(true) !== 1) {
  throw "expected Number(true) to return 1";
}
if (Number(null) !== 0) {
  throw "expected Number(null) to return 0";
}
if (Number("abc") === Number("abc")) {
  throw "expected Number('abc') to return NaN";
}
