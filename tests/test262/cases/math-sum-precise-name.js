// Derived from: test/built-ins/Math/sumPrecise/name.js
var descriptor = Object.getOwnPropertyDescriptor(Math.sumPrecise, "name");
if (descriptor.value !== "sumPrecise") {
  throw "expected Math.sumPrecise.name";
}
if (descriptor.writable || descriptor.enumerable || !descriptor.configurable) {
  throw "expected Math.sumPrecise.name descriptor attributes";
}
