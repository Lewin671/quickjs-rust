// Derived from: test/built-ins/Array/prototype/toString/length.js
if (Array.prototype.toString.length !== 0) { throw; }
if (Array.prototype.toString.propertyIsEnumerable("length")) { throw; }
