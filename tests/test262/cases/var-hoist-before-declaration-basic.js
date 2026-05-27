// Derived from: test/language/statements/variable/S12.2_A1.js
if (x !== undefined) { throw; }
var x = 1;
if (x !== 1) { throw; }
