// Derived from: test/built-ins/Math/ceil/name.js
var descriptor = Object.getOwnPropertyDescriptor(Math.ceil, "name");
if (descriptor.value !== "ceil") {
  throw "expected Math.ceil.name";
}
if (descriptor.writable || descriptor.enumerable || !descriptor.configurable) {
  throw "expected Math.ceil.name descriptor attributes";
}
