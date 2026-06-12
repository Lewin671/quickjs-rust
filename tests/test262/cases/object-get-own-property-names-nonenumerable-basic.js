// Derived from: test/built-ins/Object/prototype/S15.2.4_A2.js
var names = Object.getOwnPropertyNames(Object.prototype);
if (names.length !== 8) { throw; }
if (names[0] !== "constructor") { throw; }
if (names[1] !== "hasOwnProperty") { throw; }
if (names[2] !== "isPrototypeOf") { throw; }
if (names[3] !== "propertyIsEnumerable") { throw; }
if (names[4] !== "toLocaleString") { throw; }
if (names[5] !== "toString") { throw; }
if (names[6] !== "valueOf") { throw; }
if (names[7] !== "__proto__") { throw; }
// The __proto__ accessor is non-enumerable like the other prototype members.
if (Object.prototype.propertyIsEnumerable("__proto__")) { throw; }
