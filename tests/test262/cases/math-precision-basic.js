// Derived from: test/built-ins/Math/expm1/expm1-specialVals.js
if (Math.expm1(0) !== 0) {
  throw "expected Math.expm1(0) to return 0";
}
if (1 / Math.expm1(-0) !== -Infinity) {
  throw "expected Math.expm1(-0) to return -0";
}
if (Math.log1p(0) !== 0) {
  throw "expected Math.log1p(0) to return 0";
}
if (Math.log1p(-1) !== -Infinity) {
  throw "expected Math.log1p(-1) to return -Infinity";
}
if (Math.log1p(-2) === Math.log1p(-2)) {
  throw "expected Math.log1p(-2) to return NaN";
}
if (Math.fround(1.5) !== 1.5) {
  throw "expected Math.fround(1.5) to return 1.5";
}
if (1 / Math.fround(-0) !== -Infinity) {
  throw "expected Math.fround(-0) to return -0";
}
