// Derived from: test/built-ins/Object/preventExtensions/15.2.3.10-3-2.js
function fn() {}
Object.preventExtensions(fn);
fn.value = 1;
if (fn.value !== undefined) throw new Error("new function property should not be added");
