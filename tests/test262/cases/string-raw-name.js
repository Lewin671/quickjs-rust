// Derived from: test/built-ins/String/raw/name.js
var descriptor = Object.getOwnPropertyDescriptor(String.raw, "name");
if (descriptor.value !== "raw") {
  throw "expected String.raw.name";
}
if (descriptor.writable || descriptor.enumerable || !descriptor.configurable) {
  throw "expected String.raw.name descriptor attributes";
}
