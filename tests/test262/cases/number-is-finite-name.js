// Derived from: test/built-ins/Number/isFinite/name.js
var descriptor = Object.getOwnPropertyDescriptor(Number.isFinite, "name");
if (descriptor.value !== "isFinite") {
  throw "expected Number.isFinite.name";
}
if (descriptor.writable || descriptor.enumerable || !descriptor.configurable) {
  throw "expected Number.isFinite.name descriptor attributes";
}
