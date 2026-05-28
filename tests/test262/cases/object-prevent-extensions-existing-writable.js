// Derived from: test/built-ins/Object/preventExtensions/15.2.3.10-3-2.js
let object = { value: 1 };
Object.preventExtensions(object);
object.value = 2;
if (object.value !== 2) throw new Error("existing writable property should update");
