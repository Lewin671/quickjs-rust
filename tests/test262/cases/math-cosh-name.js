// Derived from: test/built-ins/Math/cosh/name.js
var descriptor = Object.getOwnPropertyDescriptor(Math.cosh, "name");
if (descriptor.value !== "cosh") {
  throw "expected Math.cosh.name";
}
if (descriptor.writable || descriptor.enumerable || !descriptor.configurable) {
  throw "expected Math.cosh.name descriptor attributes";
}
