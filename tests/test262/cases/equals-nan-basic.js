// Derived from: test/language/expressions/equals/S11.9.1_A4.1_T1.js
if ((NaN == NaN) !== false) { throw; }
if ((NaN == 1) !== false) { throw; }
if ((NaN == "string") !== false) { throw; }
