// Derived from: test/built-ins/Object/defineProperties/15.2.3.7-0-1.js
if (typeof Object.defineProperties !== "function") { throw; }
if (Object.defineProperties.length !== 2) { throw; }
if (Object.defineProperties.propertyIsEnumerable("length")) { throw; }
