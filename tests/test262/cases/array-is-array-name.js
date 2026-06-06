// Derived from: test/built-ins/Array/isArray/name.js
var descriptor = Object.getOwnPropertyDescriptor(Array.isArray, "name");
if (descriptor.value !== "isArray") {
  throw "expected Array.isArray.name to be isArray";
}
if (descriptor.writable || descriptor.enumerable || !descriptor.configurable) {
  throw "expected Array.isArray.name descriptor attributes";
}
