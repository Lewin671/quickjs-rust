// Derived from: test/built-ins/Array/prototype/join/length.js
if (Array.prototype.join.length !== 1) { throw; }
if (Array.prototype.join.propertyIsEnumerable("length")) { throw; }
