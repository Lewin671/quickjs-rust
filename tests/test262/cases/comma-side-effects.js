// Derived from: test/language/expressions/comma/S11.14_A2.1_T1.js
var x = 0;
var y = 0;

if ((x = 1, y = x + 2, y) !== 3) {
  throw;
}

if (x !== 1) {
  throw;
}
