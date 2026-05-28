// Derived from: test/built-ins/Object/seal/object-seal-o-is-an-array-object.js
let array = [1];
Object.seal(array);
if (Object.isSealed(array) !== true) throw new Error("array should be sealed");
if (Object.getOwnPropertyDescriptor(array, "0").configurable !== false) throw new Error("array index should be non-configurable");
