// Derived from: test/built-ins/Object/defineProperties/15.2.3.7-5-a-1.js
function fn() {}
Object.defineProperties(fn, { value: { value: 9, enumerable: true } });
if (fn.value !== 9) { throw; }
if (Object.keys(fn)[0] !== "value") { throw; }
