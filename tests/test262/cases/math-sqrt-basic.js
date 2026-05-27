// Derived from: test/built-ins/Math/sqrt/S15.8.2.17_A1.js
if (Math.sqrt(81) !== 9) {
  throw "expected Math.sqrt(81) to return 9";
}
if (Math.sqrt(2) <= 1) {
  throw "expected Math.sqrt(2) to be greater than 1";
}
if (Math.sqrt(-1) === Math.sqrt(-1)) {
  throw "expected Math.sqrt(-1) to return NaN";
}
