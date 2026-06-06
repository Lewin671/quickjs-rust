// Derived from: test/built-ins/Number/isNaN/name.js
var descriptor = Object.getOwnPropertyDescriptor(Number.isNaN, "name");
if (descriptor.value !== "isNaN") {
  throw "expected Number.isNaN.name";
}
if (descriptor.writable || descriptor.enumerable || !descriptor.configurable) {
  throw "expected Number.isNaN.name descriptor attributes";
}
