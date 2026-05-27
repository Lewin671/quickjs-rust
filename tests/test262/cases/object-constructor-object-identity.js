// Derived from: test/built-ins/Object/S15.2.2.1_A2_T5.js
var value = { x: 1 };
if (Object(value) !== value) { throw; }
if (new Object(value) !== value) { throw; }
