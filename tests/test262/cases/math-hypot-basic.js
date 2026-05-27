// Derived from: test/built-ins/Math/hypot/Math.hypot_Success_2.js
if (Math.hypot(3, 4) !== 5) {
  throw "expected Math.hypot(3, 4) to return 5";
}
if (Math.hypot() !== 0) {
  throw "expected Math.hypot() to return 0";
}
if (Math.hypot(Infinity, NaN) !== Infinity) {
  throw "expected Math.hypot with Infinity to return Infinity";
}
if (Math.hypot(NaN, 1) === Math.hypot(NaN, 1)) {
  throw "expected Math.hypot with NaN to return NaN";
}
