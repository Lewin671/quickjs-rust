// Derived from: test/built-ins/Object/create/15.2.3.5-4-1.js
var proto = { value: 42 };
var object = Object.create(proto);
if (object.value !== 42) { throw; }
