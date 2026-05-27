// Derived from: test/built-ins/Object/defineProperty/15.2.3.6-4-2.js
var object = {};
Object.defineProperty(object, "foo", { value: 1 });
var desc = Object.getOwnPropertyDescriptor(object, "foo");
if (desc.value !== 1) { throw; }
if (desc.writable !== false) { throw; }
if (desc.enumerable !== false) { throw; }
if (desc.configurable !== false) { throw; }
