// Derived from: test/language/expressions/equals/S11.9.1_A3.1.js
if ((true == 1) !== true) { throw; }
if ((false == 0) !== true) { throw; }
if ((false == "") !== true) { throw; }
if ((true == false) !== false) { throw; }
