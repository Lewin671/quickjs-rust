// Derived from: test/language/expressions/postfix-increment/S11.3.1_A2.1_T1.js
this.x = 1;
var y = this.x++;

if (y !== 1) { throw; }
if (this.x !== 2) { throw; }
