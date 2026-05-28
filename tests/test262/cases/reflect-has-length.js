// Derived from: test/built-ins/Reflect/has/length.js
if (Reflect.has.length !== 2) {
  throw "expected Reflect.has.length to be 2";
}
if (Reflect.has.propertyIsEnumerable("length")) {
  throw "expected Reflect.has.length to be non-enumerable";
}
