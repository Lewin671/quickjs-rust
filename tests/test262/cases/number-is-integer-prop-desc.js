// Derived from: test/built-ins/Number/isInteger/prop-desc.js
var descriptor = Object.getOwnPropertyDescriptor(Number, "isInteger");
if (descriptor.value !== Number.isInteger) {
  throw "expected Number.isInteger property value";
}
if (!descriptor.writable || descriptor.enumerable || !descriptor.configurable) {
  throw "expected Number.isInteger property descriptor attributes";
}
