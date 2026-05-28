// Derived from: test/built-ins/Reflect/ownKeys/length.js
if (Reflect.ownKeys.length !== 1) {
  throw "expected Reflect.ownKeys.length to be 1";
}
if (Reflect.ownKeys.propertyIsEnumerable("length")) {
  throw "expected Reflect.ownKeys.length to be non-enumerable";
}
