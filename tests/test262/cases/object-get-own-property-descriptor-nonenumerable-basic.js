// Derived from: test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-4.js
var desc = Object.getOwnPropertyDescriptor(Object.prototype, "toString");
if (typeof desc.value !== "function") { throw; }
if (desc.enumerable !== false) { throw; }
if (desc.writable !== true) { throw; }
if (desc.configurable !== true) { throw; }
