// Derived from: test/built-ins/Object/prototype/isPrototypeOf/this-value-is-in-prototype-chain-of-arg.js
var proto = { marker: 1 };
var object = Object.create(proto);
if (typeof Object.prototype.isPrototypeOf !== "function") { throw; }
if (!proto.isPrototypeOf(object)) { throw; }
if (!Object.prototype.isPrototypeOf(object)) { throw; }
if (object.isPrototypeOf(proto)) { throw; }
