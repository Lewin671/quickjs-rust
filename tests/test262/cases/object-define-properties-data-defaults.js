// Derived from: test/built-ins/Object/defineProperties/15.2.3.7-5-b-1.js
var object = {};
Object.defineProperties(object, { hidden: { value: 7 } });
var desc = Object.getOwnPropertyDescriptor(object, "hidden");
if (desc.value !== 7) { throw; }
if (desc.writable !== false) { throw; }
if (desc.enumerable !== false) { throw; }
if (desc.configurable !== false) { throw; }
