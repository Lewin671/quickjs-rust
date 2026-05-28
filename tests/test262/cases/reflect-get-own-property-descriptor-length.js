// Derived from: test/built-ins/Reflect/getOwnPropertyDescriptor/length.js
if (Reflect.getOwnPropertyDescriptor.length !== 2) {
  throw "expected Reflect.getOwnPropertyDescriptor.length to be 2";
}
if (Reflect.getOwnPropertyDescriptor.propertyIsEnumerable("length")) {
  throw "expected Reflect.getOwnPropertyDescriptor.length to be non-enumerable";
}
