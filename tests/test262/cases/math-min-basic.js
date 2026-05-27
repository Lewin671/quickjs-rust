// Derived from: test/built-ins/Math/min/S15.8.2.12_A1.js
if (Math.min(1, -9, 3) !== -9) {
  throw "expected Math.min to return the smallest argument";
}
if (Math.min() !== Infinity) {
  throw "expected Math.min() to return Infinity";
}
if (1 / Math.min(-0, 0) !== -Infinity) {
  throw "expected Math.min(-0, 0) to return -0";
}
if (Math.min(1, NaN) === Math.min(1, NaN)) {
  throw "expected Math.min with NaN to return NaN";
}
