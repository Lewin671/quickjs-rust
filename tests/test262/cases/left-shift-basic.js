// Derived from: test/language/expressions/left-shift/S11.7.1_A2.1_T1.js
if (2 << 1 !== 4) {
  throw;
}

var x = 2;
var y = 1;
if (x << y !== 4) {
  throw;
}
