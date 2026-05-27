// Derived from: test/built-ins/Object/keys/15.2.3.14-2-1.js
var keys = Object.keys([1, 2]);
if (keys.length !== 2) { throw; }
if (keys[0] !== "0") { throw; }
if (keys[1] !== "1") { throw; }
