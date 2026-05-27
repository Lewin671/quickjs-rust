// Derived from: test/language/expressions/bitwise-and/S11.10.1_A2.1_T1.js
if ((1 & 1) !== 1) {
  throw;
}

var x = 1;
if ((x & 1) !== 1) {
  throw;
}

var y = 1;
if ((x & y) !== 1) {
  throw;
}
