// Derived from: test/built-ins/Math/expm1/name.js
var descriptor = Object.getOwnPropertyDescriptor(Math.expm1, "name");
if (descriptor.value !== "expm1") {
  throw "expected Math.expm1.name";
}
if (descriptor.writable || descriptor.enumerable || !descriptor.configurable) {
  throw "expected Math.expm1.name descriptor attributes";
}
