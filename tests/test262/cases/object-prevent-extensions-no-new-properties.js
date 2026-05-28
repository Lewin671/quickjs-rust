// Derived from: test/built-ins/Object/preventExtensions/15.2.3.10-3-10.js
let object = {};
Object.preventExtensions(object);
object.value = 1;
if (object.value !== undefined) throw new Error("new property should not be added");
