// Derived from: test/built-ins/Math/asin/name.js
var descriptor = Object.getOwnPropertyDescriptor(Math.asin, "name");
if (descriptor.value !== "asin") {
  throw "expected Math.asin.name";
}
if (descriptor.writable || descriptor.enumerable || !descriptor.configurable) {
  throw "expected Math.asin.name descriptor attributes";
}
