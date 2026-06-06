// Derived from: test/built-ins/Math/pow/name.js
var descriptor = Object.getOwnPropertyDescriptor(Math.pow, "name");
if (descriptor.value !== "pow") {
  throw "expected Math.pow.name";
}
if (descriptor.writable || descriptor.enumerable || !descriptor.configurable) {
  throw "expected Math.pow.name descriptor attributes";
}
