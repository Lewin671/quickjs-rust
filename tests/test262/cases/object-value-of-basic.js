// Derived from: test/built-ins/Object/prototype/valueOf/S15.2.4.4_A1_T7.js
var object = { value: 1 };
if (typeof Object.prototype.valueOf !== "function") { throw; }
if (object.valueOf() !== object) { throw; }
