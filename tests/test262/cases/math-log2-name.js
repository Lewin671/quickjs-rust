// Derived from: test/built-ins/Math/log2/name.js
var descriptor = Object.getOwnPropertyDescriptor(Math.log2, "name");
if (descriptor.value !== "log2") {
  throw "expected Math.log2.name";
}
if (descriptor.writable || descriptor.enumerable || !descriptor.configurable) {
  throw "expected Math.log2.name descriptor attributes";
}
