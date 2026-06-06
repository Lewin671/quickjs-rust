// Derived from: test/built-ins/Math/min/name.js
var descriptor = Object.getOwnPropertyDescriptor(Math.min, "name");
if (descriptor.value !== "min") {
  throw "expected Math.min.name";
}
if (descriptor.writable || descriptor.enumerable || !descriptor.configurable) {
  throw "expected Math.min.name descriptor attributes";
}
