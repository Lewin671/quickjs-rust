// Derived from: test/built-ins/Object/freeze/15.2.3.9-2-a-1.js
var object = { value: 1 };
Object.freeze(object);
if (Object.getOwnPropertyDescriptor(object, "value").configurable !== false) {
  throw new Error("frozen data property should be non-configurable");
}
