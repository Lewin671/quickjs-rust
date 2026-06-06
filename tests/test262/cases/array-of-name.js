// Derived from: test/built-ins/Array/of/name.js
var descriptor = Object.getOwnPropertyDescriptor(Array.of, "name");
if (descriptor.value !== "of") {
  throw "expected Array.of.name";
}
if (descriptor.writable || descriptor.enumerable || !descriptor.configurable) {
  throw "expected Array.of.name descriptor attributes";
}
