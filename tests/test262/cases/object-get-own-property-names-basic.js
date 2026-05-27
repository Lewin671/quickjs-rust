// Derived from: test/built-ins/Object/getOwnPropertyNames/15.2.3.4-3-1.js
var object = { prop1: 1001 };
var names = Object.getOwnPropertyNames(object);
if (names.length !== 1) { throw; }
if (names[0] !== "prop1") { throw; }
