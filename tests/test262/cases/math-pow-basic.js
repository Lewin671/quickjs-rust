// Derived from: test/built-ins/Math/pow/applying-the-exp-operator_A1.js
if (Math.pow(2, 8) !== 256) {
  throw "expected Math.pow(2, 8) to return 256";
}
if (Math.pow(-2, 3) !== -8) {
  throw "expected Math.pow(-2, 3) to return -8";
}
if (Math.pow(2, NaN) === Math.pow(2, NaN)) {
  throw "expected Math.pow with NaN exponent to return NaN";
}
