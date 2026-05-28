// Derived from: test/built-ins/Object/is/length.js
if (Object.is.length !== 2) {
  throw "Object.is.length should be 2";
}
if (Object.is.propertyIsEnumerable("length")) {
  throw "Object.is.length should be non-enumerable";
}
