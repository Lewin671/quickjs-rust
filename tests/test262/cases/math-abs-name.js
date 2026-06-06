// Derived from: test/built-ins/Math/abs/name.js
var descriptor = Object.getOwnPropertyDescriptor(Math.abs, "name");
if (descriptor.value !== "abs") {
  throw "expected Math.abs.name";
}
if (descriptor.writable || descriptor.enumerable || !descriptor.configurable) {
  throw "expected Math.abs.name descriptor attributes";
}
