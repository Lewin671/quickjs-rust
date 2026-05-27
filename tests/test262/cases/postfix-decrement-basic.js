// Derived from: test/language/expressions/postfix-decrement/S11.3.2_A4_T1.js
var x = true;
var y = x--;
if (y !== 1) { throw; }
if (x !== 0) { throw; }
