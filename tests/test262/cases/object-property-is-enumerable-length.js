// Derived from: test/built-ins/Object/prototype/propertyIsEnumerable/S15.2.4.7_A8.js
if (Object.prototype.propertyIsEnumerable.length !== 1) { throw; }
if (!Object.prototype.propertyIsEnumerable.hasOwnProperty("length")) { throw; }
if (Object.prototype.propertyIsEnumerable.propertyIsEnumerable("length")) { throw; }
