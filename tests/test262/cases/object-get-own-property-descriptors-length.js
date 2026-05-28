// Derived from: test/built-ins/Object/getOwnPropertyDescriptors/function-length.js
if (Object.getOwnPropertyDescriptors.length !== 1) {
  throw "Object.getOwnPropertyDescriptors.length should be 1";
}
if (Object.getOwnPropertyDescriptors.propertyIsEnumerable("length")) {
  throw "Object.getOwnPropertyDescriptors.length should be non-enumerable";
}
