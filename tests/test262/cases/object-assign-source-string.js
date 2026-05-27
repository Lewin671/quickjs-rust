// Derived from: test/built-ins/Object/assign/Source-String.js
var target = new Object();
var result = Object.assign(target, "123");
if (result[0] !== "1") { throw; }
if (result[1] !== "2") { throw; }
if (result[2] !== "3") { throw; }
