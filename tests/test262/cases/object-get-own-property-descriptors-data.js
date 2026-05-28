// Derived from: test/built-ins/Object/getOwnPropertyDescriptors/inherited-properties-omitted.js
var object = { value: 1 };
Object.defineProperty(object, "hidden", { value: 2 });
var descriptors = Object.getOwnPropertyDescriptors(object);
if (descriptors.value.value !== 1 || descriptors.value.enumerable !== true) {
  throw "Object.getOwnPropertyDescriptors should expose enumerable data properties";
}
if (descriptors.hidden.value !== 2 || descriptors.hidden.enumerable !== false) {
  throw "Object.getOwnPropertyDescriptors should expose non-enumerable data properties";
}
