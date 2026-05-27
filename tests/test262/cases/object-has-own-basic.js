// Derived from: test/built-ins/Object/hasOwn/hasown.js
var object = { value: 1 };
if (typeof Object.hasOwn !== "function") { throw; }
if (!Object.hasOwn(Object, "hasOwn")) { throw; }
if (!Object.hasOwn(object, "value")) { throw; }
