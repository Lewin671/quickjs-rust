// Derived from: test/language/expressions/equals/S11.9.1_A5.1.js
if (("1" == 1) !== true) { throw; }
if ((1 == "1") !== true) { throw; }
if (("x" == 1) !== false) { throw; }
