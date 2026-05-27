// Derived from: test/language/expressions/strict-equals/S11.9.4_A2.1_T1.js
if (!(1 === 1)) { throw; }
var x = 1;
if (!(x === 1)) { throw; }
var y = 1;
if (!(1 === y)) { throw; }
if (!(x === y)) { throw; }
