// Derived from: test/built-ins/Math/log10/name.js
var descriptor = Object.getOwnPropertyDescriptor(Math.log10, "name");
if (descriptor.value !== "log10") {
  throw "expected Math.log10.name";
}
if (descriptor.writable || descriptor.enumerable || !descriptor.configurable) {
  throw "expected Math.log10.name descriptor attributes";
}
