// Derived from: test/built-ins/Object/keys/15.2.3.14-3-1.js
var object = { x: 1, y: 2 };
var keys = Object.keys(object);
if (keys.length !== 2) { throw; }
if (keys[0] !== "x") { throw; }
if (keys[1] !== "y") { throw; }
