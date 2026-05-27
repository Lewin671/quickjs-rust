// Derived from: test/language/expressions/compound-assignment/S11.13.2_A4.1_T2.1.js
var x = true;
x *= 1;
if (x !== 1) { throw; }
x += 2;
if (x !== 3) { throw; }
x -= true;
if (x !== 2) { throw; }
x /= 2;
if (x !== 1) { throw; }
x %= 1;
if (x !== 0) { throw; }
