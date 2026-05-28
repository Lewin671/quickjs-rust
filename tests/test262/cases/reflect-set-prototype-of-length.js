// Derived from: test/built-ins/Reflect/setPrototypeOf/length.js
if (Reflect.setPrototypeOf.length !== 2) {
  throw "expected Reflect.setPrototypeOf.length to be 2";
}
if (Reflect.setPrototypeOf.propertyIsEnumerable("length")) {
  throw "expected Reflect.setPrototypeOf.length to be non-enumerable";
}
