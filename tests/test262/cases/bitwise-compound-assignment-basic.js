// Derived from: test/language/expressions/compound-assignment/S11.13.2_A3.2_T9.js
var x = 5;

if ((x &= 3) !== 1) {
  throw;
}

if (x !== 1) {
  throw;
}

x = 5;
if ((x ^= 3) !== 6) {
  throw;
}

x = 5;
if ((x |= 2) !== 7) {
  throw;
}
