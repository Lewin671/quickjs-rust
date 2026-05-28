// Derived from: test/built-ins/Reflect/getPrototypeOf/length.js
if (Reflect.getPrototypeOf.length !== 1) {
  throw "expected Reflect.getPrototypeOf.length to be 1";
}
if (Reflect.getPrototypeOf.propertyIsEnumerable("length")) {
  throw "expected Reflect.getPrototypeOf.length to be non-enumerable";
}
