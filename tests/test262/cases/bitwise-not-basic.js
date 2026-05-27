// Derived from: test/language/expressions/bitwise-not/S11.4.8_A3_T1.js
if (~false !== -1) {
  throw;
}

if (~true !== -2) {
  throw;
}
