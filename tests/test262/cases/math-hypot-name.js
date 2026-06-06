// Derived from: test/built-ins/Math/hypot/name.js
var descriptor = Object.getOwnPropertyDescriptor(Math.hypot, "name");
if (descriptor.value !== "hypot") {
  throw "expected Math.hypot.name";
}
if (descriptor.writable || descriptor.enumerable || !descriptor.configurable) {
  throw "expected Math.hypot.name descriptor attributes";
}
