// Derived from: test/built-ins/Object/assign/Source-Null-Undefined.js
var target = { a: 1 };
var result = Object.assign(target, null, undefined);
if (result !== target) { throw; }
if (result.a !== 1) { throw; }
if (Object.keys(result).length !== 1) { throw; }
