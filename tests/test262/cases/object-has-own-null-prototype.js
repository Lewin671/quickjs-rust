// Derived from: test/built-ins/Object/hasOwn/hasown.js
var object = Object.create(null, { value: { value: 1 } });
if (!Object.hasOwn(object, "value")) { throw; }
