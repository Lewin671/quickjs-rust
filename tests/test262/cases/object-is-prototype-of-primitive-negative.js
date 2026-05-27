// Derived from: test/built-ins/Object/prototype/isPrototypeOf/null-this-and-primitive-arg-returns-false.js
if (Object.prototype.isPrototypeOf(undefined) !== false) { throw; }
if (Object.prototype.isPrototypeOf(null) !== false) { throw; }
if (Object.prototype.isPrototypeOf(false) !== false) { throw; }
if (Object.prototype.isPrototypeOf("") !== false) { throw; }
if (Object.prototype.isPrototypeOf(10) !== false) { throw; }
