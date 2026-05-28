// Derived from: test/built-ins/Object/seal/object-seal-p-is-own-data-property.js
let object = { value: 1 };
Object.seal(object);
object.value = 2;
if (object.value !== 2) throw new Error("sealed writable property should update");
