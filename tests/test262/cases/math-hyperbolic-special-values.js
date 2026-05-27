// Derived from: test/built-ins/Math/asinh/asinh-specialVals.js
if (Math.asinh(NaN) === Math.asinh(NaN)) {
  throw "expected Math.asinh(NaN) to return NaN";
}
if (1 / Math.asinh(-0) !== -Infinity) {
  throw "expected Math.asinh(-0) to return -0";
}
if (Math.atanh(-1) !== -Infinity) {
  throw "expected Math.atanh(-1) to return -Infinity";
}
if (Math.atanh(2) === Math.atanh(2)) {
  throw "expected Math.atanh(2) to return NaN";
}
if (Math.cosh(Infinity) !== Infinity) {
  throw "expected Math.cosh(Infinity) to return Infinity";
}
if (1 / Math.sinh(-0) !== -Infinity) {
  throw "expected Math.sinh(-0) to return -0";
}
if (1 / Math.tanh(-0) !== -Infinity) {
  throw "expected Math.tanh(-0) to return -0";
}
