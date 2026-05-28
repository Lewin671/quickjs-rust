// Derived from: test/built-ins/Object/freeze/15.2.3.9-2-b-i-1.js
var object = { value: 1 };
Object.freeze(object);
object.value = 2;
if (object.value !== 1) throw new Error("frozen writable property should not update");
