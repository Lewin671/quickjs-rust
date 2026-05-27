// Derived from: test/built-ins/Math/round/S15.8.2.15_A1.js
if (Math.round(1.5) !== 2) {
  throw "expected Math.round(1.5) to return 2";
}
if (Math.round(-1.5) !== -1) {
  throw "expected Math.round(-1.5) to return -1";
}
if (1 / Math.round(-0.4) !== -Infinity) {
  throw "expected Math.round(-0.4) to return -0";
}
if (Math.round(NaN) === Math.round(NaN)) {
  throw "expected Math.round(NaN) to return NaN";
}
