// Derived from: test/built-ins/Array/is-a-constructor.js
if (!([] instanceof Array)) { throw; }
if (!Array.prototype.isPrototypeOf([])) { throw; }
