// Derived from: test/built-ins/Math/atan/name.js
var descriptor = Object.getOwnPropertyDescriptor(Math.atan, "name");
if (descriptor.value !== "atan") {
  throw "expected Math.atan.name";
}
if (descriptor.writable || descriptor.enumerable || !descriptor.configurable) {
  throw "expected Math.atan.name descriptor attributes";
}
