// Derived from: test/language/statements/variable/S12.2_A1.js
var x, y = true, z = y;
if (x !== undefined) { throw; }
if (y !== true) { throw; }
if (z !== true) { throw; }

let a = 1, b = 2;
if (a + b !== 3) { throw; }

const c = 4, d = 5;
if (c + d !== 9) { throw; }
