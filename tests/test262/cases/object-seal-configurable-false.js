// Derived from: test/built-ins/Object/seal/object-seal-p-is-own-data-property.js
let object = { value: 1 };
Object.seal(object);
if (Object.getOwnPropertyDescriptor(object, "value").configurable !== false) {
  throw new Error("sealed data property should be non-configurable");
}
