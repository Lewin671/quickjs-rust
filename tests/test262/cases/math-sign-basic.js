// Derived from: test/built-ins/Math/sign/sign-specialVals.js
if (Math.sign(-1) !== -1) {
  throw "expected Math.sign(-1) to return -1";
}
if (Math.sign(1) !== 1) {
  throw "expected Math.sign(1) to return 1";
}
if (1 / Math.sign(-0) !== -Infinity) {
  throw "expected Math.sign(-0) to return -0";
}
if (Math.sign(NaN) === Math.sign(NaN)) {
  throw "expected Math.sign(NaN) to return NaN";
}
