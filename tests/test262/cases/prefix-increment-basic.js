// Derived from: test/language/expressions/prefix-increment/S11.4.4_A4_T1.js
var x = false;
if (++x !== 1) { throw; }
if (x !== 1) { throw; }
