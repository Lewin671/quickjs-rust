// Derived from: test/language/expressions/bitwise-or/S11.10.3_A2.1_T1.js
if ((1 | 0) !== 1) {
  throw;
}

var x = 1;
if ((x | 0) !== 1) {
  throw;
}

var y = 0;
if ((x | y) !== 1) {
  throw;
}
