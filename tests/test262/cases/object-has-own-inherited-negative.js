// Derived from: test/built-ins/Object/hasOwn/hasown_inherited_exists.js
var base = { value: 42 };
var object = Object.create(base);
if (Object.hasOwn(object, "value")) { throw; }
