// Derived from: test/built-ins/Object/defineProperty/15.2.3.6-0-1.js
if (typeof Object.defineProperty !== "function") { throw; }
if (Object.defineProperty.length !== 3) { throw; }
if (Object.defineProperty.propertyIsEnumerable("length")) { throw; }
