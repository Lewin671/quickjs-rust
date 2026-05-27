// Derived from: test/built-ins/Math/abs/absolute-value.js
if (Math.abs(-7) !== 7) {
  throw "expected Math.abs(-7) to return 7";
}
if (Math.abs(7) !== 7) {
  throw "expected Math.abs(7) to return 7";
}
if (1 / Math.abs(-0) !== Infinity) {
  throw "expected Math.abs(-0) to return +0";
}
