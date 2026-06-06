// Derived from: test/built-ins/Math/max/name.js
var descriptor = Object.getOwnPropertyDescriptor(Math.max, "name");
if (descriptor.value !== "max") {
  throw "expected Math.max.name";
}
if (descriptor.writable || descriptor.enumerable || !descriptor.configurable) {
  throw "expected Math.max.name descriptor attributes";
}
