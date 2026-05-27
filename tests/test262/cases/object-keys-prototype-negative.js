// Derived from: test/built-ins/Object/keys/15.2.3.14-3-1.js
var object = Object.create({ inherited: 1 });
var keys = Object.keys(object);
if (keys.length !== 0) { throw; }
