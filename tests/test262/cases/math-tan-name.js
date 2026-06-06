// Derived from: test/built-ins/Math/tan/name.js
var descriptor = Object.getOwnPropertyDescriptor(Math.tan, "name");
if (descriptor.value !== "tan") {
  throw "expected Math.tan.name";
}
if (descriptor.writable || descriptor.enumerable || !descriptor.configurable) {
  throw "expected Math.tan.name descriptor attributes";
}
