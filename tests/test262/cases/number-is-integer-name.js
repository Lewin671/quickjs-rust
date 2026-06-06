// Derived from: test/built-ins/Number/isInteger/name.js
var descriptor = Object.getOwnPropertyDescriptor(Number.isInteger, "name");
if (descriptor.value !== "isInteger") {
  throw "expected Number.isInteger.name";
}
if (descriptor.writable || descriptor.enumerable || !descriptor.configurable) {
  throw "expected Number.isInteger.name descriptor attributes";
}
