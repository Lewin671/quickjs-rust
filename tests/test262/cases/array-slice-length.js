// Derived from: test/built-ins/Array/prototype/slice/length.js
if (Array.prototype.slice.length !== 2) { throw; }
if (Array.prototype.slice.propertyIsEnumerable("length")) { throw; }
