// Derived from: test/language/expressions/void/S11.4.2_A2_T1.js
if (void 0 !== undefined) {
  throw;
}

var x = 0;
if (void x !== undefined) {
  throw;
}
