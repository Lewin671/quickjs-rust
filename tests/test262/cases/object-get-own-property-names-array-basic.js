// Derived from: test/built-ins/Object/getOwnPropertyNames/15.2.3.4-2-1.js
var names = Object.getOwnPropertyNames([1, 2]);
if (names.length !== 3) { throw; }
if (names[0] !== "0") { throw; }
if (names[2] !== "length") { throw; }
