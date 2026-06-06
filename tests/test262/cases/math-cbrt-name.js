// Derived from: test/built-ins/Math/cbrt/name.js
var descriptor = Object.getOwnPropertyDescriptor(Math.cbrt, "name");
if (descriptor.value !== "cbrt") {
  throw "expected Math.cbrt.name";
}
if (descriptor.writable || descriptor.enumerable || !descriptor.configurable) {
  throw "expected Math.cbrt.name descriptor attributes";
}
