// Derived from: test/built-ins/Object/assign/assign-length.js
if (typeof Object.assign !== "function") { throw; }
if (Object.assign.length !== 2) { throw; }
if (Object.assign.propertyIsEnumerable("length")) { throw; }
