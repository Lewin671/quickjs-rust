// Derived from: test/built-ins/Object/freeze/15.2.3.9-2-d-1.js
function fn() {}
fn.value = 1;
Object.freeze(fn);
fn.value = 2;
if (Object.isFrozen(fn) !== true) throw new Error("function should be frozen");
if (fn.value !== 1) throw new Error("frozen function property should not update");
