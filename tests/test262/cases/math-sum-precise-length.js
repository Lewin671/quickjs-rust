// Derived from: test/built-ins/Math/sumPrecise/length.js
var descriptor = Object.getOwnPropertyDescriptor(Math.sumPrecise, "length");
if (descriptor.value !== 1) {
  throw "expected Math.sumPrecise.length";
}
if (descriptor.writable || descriptor.enumerable || !descriptor.configurable) {
  throw "expected Math.sumPrecise.length descriptor attributes";
}
