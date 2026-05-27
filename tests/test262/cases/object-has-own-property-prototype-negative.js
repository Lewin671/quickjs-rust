// Derived from: test/built-ins/Object/prototype/hasOwnProperty/8.12.1-1_23.js
var proto = { value: 1 };
var object = Object.create(proto);
if (object.value !== 1) { throw; }
if (object.hasOwnProperty("value")) { throw; }
