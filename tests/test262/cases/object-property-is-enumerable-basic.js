// Derived from: test/built-ins/Object/prototype/propertyIsEnumerable/S15.2.4.7_A2_T1.js
var object = { value: 1 };
if (typeof Object.prototype.propertyIsEnumerable !== "function") { throw; }
if (!object.propertyIsEnumerable("value")) { throw; }
if (Object.prototype.propertyIsEnumerable("propertyIsEnumerable")) { throw; }
