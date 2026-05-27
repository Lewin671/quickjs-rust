// Derived from: test/built-ins/Object/defineProperty/15.2.3.6-4-2.js
function fn() {}
Object.defineProperty(fn, "foo", { value: 1, enumerable: true });
if (fn.foo !== 1) { throw; }
if (Object.keys(fn)[0] !== "foo") { throw; }
