// Derived from: test/built-ins/parseInt/S15.1.2.2_A2_T1.js
if (parseInt("xyz") === parseInt("xyz")) {
  throw "expected parseInt invalid input to return NaN";
}
if (parseInt("10", 37) === parseInt("10", 37)) {
  throw "expected parseInt invalid radix to return NaN";
}
