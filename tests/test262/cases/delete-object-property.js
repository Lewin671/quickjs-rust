// Derived from: test/language/expressions/delete/S8.12.7_A3.js
var map = { red: 1, 2: 2 };
if (delete map.red !== true) { throw; }
if (map.red !== undefined) { throw; }
if (delete map[2] !== true) { throw; }
if (map["2"] !== undefined) { throw; }
