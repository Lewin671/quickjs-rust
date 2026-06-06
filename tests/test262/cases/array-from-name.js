// Derived from: test/built-ins/Array/from/Array.from-name.js
var descriptor = Object.getOwnPropertyDescriptor(Array.from, "name");
if (descriptor.value !== "from") {
  throw "expected Array.from.name";
}
if (descriptor.writable || descriptor.enumerable || !descriptor.configurable) {
  throw "expected Array.from.name descriptor attributes";
}
