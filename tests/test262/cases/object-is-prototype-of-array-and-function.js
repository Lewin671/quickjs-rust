// Derived from: test/built-ins/Object/prototype/isPrototypeOf/this-value-is-in-prototype-chain-of-arg.js
function F() {}
if (!Object.prototype.isPrototypeOf([1, 2])) { throw; }
if (!Object.prototype.isPrototypeOf(F)) { throw; }
if (F.prototype.isPrototypeOf(F)) { throw; }
