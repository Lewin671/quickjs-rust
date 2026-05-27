// Derived from: test/language/expressions/void/S11.4.2_A4_T1.js
var x = false;

if (void (x = true) !== undefined) {
  throw;
}

if (x !== true) {
  throw;
}
