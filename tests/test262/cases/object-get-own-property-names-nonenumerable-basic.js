// Derived from: test/built-ins/Object/prototype/S15.2.4_A2.js
var names = Object.getOwnPropertyNames(Object.prototype);
if (names.length !== 12) { throw; }
if (names[0] !== "constructor") { throw; }
if (names[1] !== "hasOwnProperty") { throw; }
if (names[2] !== "isPrototypeOf") { throw; }
if (names[3] !== "propertyIsEnumerable") { throw; }
if (names[4] !== "toLocaleString") { throw; }
if (names[5] !== "toString") { throw; }
if (names[6] !== "valueOf") { throw; }
if (names[7] !== "__defineGetter__") { throw; }
if (names[8] !== "__defineSetter__") { throw; }
if (names[9] !== "__lookupGetter__") { throw; }
if (names[10] !== "__lookupSetter__") { throw; }
if (names[11] !== "__proto__") { throw; }
// Annex B accessor helpers and the __proto__ accessor are non-enumerable like
// the other prototype members.
if (Object.prototype.propertyIsEnumerable("__defineGetter__")) { throw; }
if (Object.prototype.propertyIsEnumerable("__defineSetter__")) { throw; }
if (Object.prototype.propertyIsEnumerable("__lookupGetter__")) { throw; }
if (Object.prototype.propertyIsEnumerable("__lookupSetter__")) { throw; }
if (Object.prototype.propertyIsEnumerable("__proto__")) { throw; }
