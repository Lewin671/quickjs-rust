// Derived from: test/built-ins/Math/sumPrecise/prop-desc.js
var descriptor = Object.getOwnPropertyDescriptor(Math, "sumPrecise");
if (descriptor.value !== Math.sumPrecise) {
  throw "expected Math.sumPrecise property value";
}
if (!descriptor.writable || descriptor.enumerable || !descriptor.configurable) {
  throw "expected Math.sumPrecise property descriptor attributes";
}
