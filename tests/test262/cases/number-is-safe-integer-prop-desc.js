// Derived from: test/built-ins/Number/isSafeInteger/prop-desc.js
var descriptor = Object.getOwnPropertyDescriptor(Number, "isSafeInteger");
if (descriptor.value !== Number.isSafeInteger) {
  throw "expected Number.isSafeInteger property value";
}
if (!descriptor.writable || descriptor.enumerable || !descriptor.configurable) {
  throw "expected Number.isSafeInteger property descriptor attributes";
}
