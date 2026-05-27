// Derived from: test/language/expressions/less-than/S11.8.1_A2.1_T1.js
if ((1 < 2) !== true) { throw; }
var x = 1;
if ((x < 2) !== true) { throw; }
var y = 2;
if ((1 < y) !== true) { throw; }
if ((x < y) !== true) { throw; }
