// Derived from: test/language/expressions/logical-and/S11.11.1_A2.1_T1.js
if ((false && true) !== false) { throw; }
if ((true && false) !== false) { throw; }
var x = false;
if ((x && true) !== false) { throw; }
var y = true;
if ((x && y) !== false) { throw; }
