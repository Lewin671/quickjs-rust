// Derived from: test/language/expressions/object/S11.1.5_A3.js
var object = { 0: 1, "1": "x", o: {} };
if (object[0] !== 1) { throw; }
if (object["1"] !== "x") { throw; }
if (typeof object.o !== "object") { throw; }
