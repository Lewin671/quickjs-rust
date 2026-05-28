// Derived from: test/built-ins/parseFloat/S15.1.2.3_A2_T1.js
if (parseFloat("xyz") === parseFloat("xyz")) {
  throw "expected parseFloat invalid input to return NaN";
}
