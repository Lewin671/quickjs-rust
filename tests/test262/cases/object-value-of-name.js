// Derived from: test/built-ins/Object/prototype/valueOf/name.js
var descriptor = Object.getOwnPropertyDescriptor(Object.prototype.valueOf, "name");
if (descriptor.value !== "valueOf") { throw; }
if (descriptor.writable !== false) { throw; }
if (descriptor.enumerable !== false) { throw; }
if (descriptor.configurable !== true) { throw; }
