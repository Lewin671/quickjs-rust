// Derived from: test/built-ins/Array/prototype/concat/length.js
if (Array.prototype.concat.length !== 1) { throw; }
if (Array.prototype.concat.propertyIsEnumerable("length")) { throw; }
