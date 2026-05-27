// Derived from: test/built-ins/Object/prototype/isPrototypeOf/length.js
if (Object.prototype.isPrototypeOf.length !== 1) { throw; }
if (!Object.prototype.isPrototypeOf.hasOwnProperty("length")) { throw; }
if (Object.prototype.isPrototypeOf.propertyIsEnumerable("length")) { throw; }
