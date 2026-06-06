// Derived from: test/built-ins/Number/isFinite/prop-desc.js
var descriptor = Object.getOwnPropertyDescriptor(Number, "isFinite");
if (descriptor.value !== Number.isFinite) {
  throw "expected Number.isFinite property value";
}
if (!descriptor.writable || descriptor.enumerable || !descriptor.configurable) {
  throw "expected Number.isFinite property descriptor attributes";
}
