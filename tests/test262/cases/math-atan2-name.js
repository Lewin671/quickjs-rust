// Derived from: test/built-ins/Math/atan2/name.js
var descriptor = Object.getOwnPropertyDescriptor(Math.atan2, "name");
if (descriptor.value !== "atan2") {
  throw "expected Math.atan2.name";
}
if (descriptor.writable || descriptor.enumerable || !descriptor.configurable) {
  throw "expected Math.atan2.name descriptor attributes";
}
