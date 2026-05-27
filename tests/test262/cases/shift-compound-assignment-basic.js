// Derived from: test/language/expressions/compound-assignment/S11.13.2_A3.2_T6.js
var x = 2;

if ((x <<= 3) !== 16) {
  throw;
}

x = -8;
if ((x >>= 1) !== -4) {
  throw;
}

x = 1;
if ((x >>>= 1) !== 0) {
  throw;
}
