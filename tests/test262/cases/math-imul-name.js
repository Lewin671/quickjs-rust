// Derived from: test/built-ins/Math/imul/name.js
var descriptor = Object.getOwnPropertyDescriptor(Math.imul, "name");
if (descriptor.value !== "imul") {
  throw "expected Math.imul.name";
}
if (descriptor.writable || descriptor.enumerable || !descriptor.configurable) {
  throw "expected Math.imul.name descriptor attributes";
}
