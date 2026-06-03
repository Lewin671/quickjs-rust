// Derived from: test/built-ins/Number/MIN_VALUE/value.js
if (typeof Number.MIN_VALUE !== "number") {
  throw "expected Number.MIN_VALUE to be a number";
}
if (!(Number.MIN_VALUE > 0)) {
  throw "expected Number.MIN_VALUE to be positive";
}
if (!(Number.MIN_VALUE < Number.EPSILON)) {
  throw "expected Number.MIN_VALUE to be smaller than Number.EPSILON";
}
if (Number.MIN_VALUE / 2 !== 0) {
  throw "expected Number.MIN_VALUE divided by 2 to underflow to zero";
}
