// Derived from: test/built-ins/Math/max/S15.8.2.11_A1.js
if (Math.max(1, 9, 3) !== 9) {
  throw "expected Math.max to return the largest argument";
}
if (Math.max() !== -Infinity) {
  throw "expected Math.max() to return -Infinity";
}
if (1 / Math.max(-0, 0) !== Infinity) {
  throw "expected Math.max(-0, 0) to return +0";
}
if (Math.max(1, NaN) === Math.max(1, NaN)) {
  throw "expected Math.max with NaN to return NaN";
}
