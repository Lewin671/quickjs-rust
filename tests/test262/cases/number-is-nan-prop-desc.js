// Derived from: test/built-ins/Number/isNaN/prop-desc.js
var descriptor = Object.getOwnPropertyDescriptor(Number, "isNaN");
if (descriptor.value !== Number.isNaN) {
  throw "expected Number.isNaN property value";
}
if (!descriptor.writable || descriptor.enumerable || !descriptor.configurable) {
  throw "expected Number.isNaN property descriptor attributes";
}
