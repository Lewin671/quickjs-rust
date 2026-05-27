// Derived from: test/language/expressions/comma/S11.14_A2.1_T1.js
if ((1, 2) !== 2) {
  throw;
}

var x = 1;
if ((x, 2) !== 2) {
  throw;
}

var y = 2;
if ((1, y) !== 2) {
  throw;
}

if ((x, y) !== 2) {
  throw;
}
