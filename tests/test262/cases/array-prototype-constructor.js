// Derived from: test/built-ins/Array/prototype/constructor.js
if (Array.prototype.constructor !== Array) { throw; }
if (Array.prototype.propertyIsEnumerable("constructor")) { throw; }
