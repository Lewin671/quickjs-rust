// Derived from: test/built-ins/Object/prototype/hasOwnProperty/S15.2.4.5_A1_T1.js
var object = { value: 1 };
if (typeof Object.prototype.hasOwnProperty !== "function") { throw; }
if (!Object.prototype.hasOwnProperty("hasOwnProperty")) { throw; }
if (!object.hasOwnProperty("value")) { throw; }
