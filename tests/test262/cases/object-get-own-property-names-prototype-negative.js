// Derived from: test/built-ins/Object/getOwnPropertyNames/15.2.3.4-4-36.js
var object = Object.create({ parent: "parent" });
var names = Object.getOwnPropertyNames(object);
if (names.length !== 0) { throw; }
