// Derived from: test/language/expressions/postfix-increment/S11.3.1_A4_T1.js
var x = false;
var y = x++;
if (y !== 0) { throw; }
if (x !== 1) { throw; }
