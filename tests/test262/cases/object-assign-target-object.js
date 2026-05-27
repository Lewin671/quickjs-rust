// Derived from: test/built-ins/Object/assign/Target-Object.js
var target = { foo: 1 };
var result = Object.assign(target, { a: 2 });
if (result !== target) { throw; }
if (result.foo !== 1) { throw; }
if (result.a !== 2) { throw; }
