// Derived from: test/built-ins/Array/isArray/15.4.3.2-0-2.js
if (Array.isArray.length !== 1) { throw; }
if (Array.isArray.propertyIsEnumerable("length")) { throw; }
