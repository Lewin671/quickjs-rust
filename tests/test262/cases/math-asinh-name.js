// Derived from: test/built-ins/Math/asinh/name.js
var descriptor = Object.getOwnPropertyDescriptor(Math.asinh, "name");
if (descriptor.value !== "asinh") {
  throw "expected Math.asinh.name";
}
if (descriptor.writable || descriptor.enumerable || !descriptor.configurable) {
  throw "expected Math.asinh.name descriptor attributes";
}
