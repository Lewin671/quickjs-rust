// Derived from: test/built-ins/Math/acosh/arg-is-one.js
if (Math.acosh(1) !== 0) {
  throw "expected Math.acosh(1) to return 0";
}
if (Math.asinh(0) !== 0) {
  throw "expected Math.asinh(0) to return 0";
}
if (Math.atanh(0) !== 0) {
  throw "expected Math.atanh(0) to return 0";
}
if (Math.atanh(1) !== Infinity) {
  throw "expected Math.atanh(1) to return Infinity";
}
if (Math.cosh(0) !== 1) {
  throw "expected Math.cosh(0) to return 1";
}
if (Math.sinh(0) !== 0) {
  throw "expected Math.sinh(0) to return 0";
}
if (Math.tanh(Infinity) !== 1) {
  throw "expected Math.tanh(Infinity) to return 1";
}
