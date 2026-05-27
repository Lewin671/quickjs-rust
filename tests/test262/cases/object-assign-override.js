// Derived from: test/built-ins/Object/assign/Override.js
var target = { a: 1 };
var result = Object.assign(target, "1a2", { a: "c" }, undefined, { b: 6 }, null, 125, { a: 5 });
if (Object.getOwnPropertyNames(result).length !== 5) { throw; }
if (result.a !== 5) { throw; }
if (result[0] !== "1") { throw; }
if (result[1] !== "a") { throw; }
if (result[2] !== "2") { throw; }
if (result.b !== 6) { throw; }
