// Derived from: test/built-ins/Object/hasOwn/length.js
if (Object.hasOwn.length !== 2) { throw; }
if (Object.hasOwn.propertyIsEnumerable("length")) { throw; }
