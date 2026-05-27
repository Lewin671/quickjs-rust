// Derived from: test/built-ins/Object/keys/15.2.3.14-1-3.js
var keys = Object.keys("abc");
if (keys.length !== 3) { throw; }
if (keys[0] !== "0") { throw; }
if (keys[2] !== "2") { throw; }
