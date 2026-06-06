// Derived from: test/built-ins/Number/isSafeInteger/name.js
var descriptor = Object.getOwnPropertyDescriptor(Number.isSafeInteger, "name");
if (descriptor.value !== "isSafeInteger") {
  throw "expected Number.isSafeInteger.name";
}
if (descriptor.writable || descriptor.enumerable || !descriptor.configurable) {
  throw "expected Number.isSafeInteger.name descriptor attributes";
}
