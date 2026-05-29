// Derived from: test/built-ins/Math/random/prop-desc.js
var descriptor = Object.getOwnPropertyDescriptor(Math, "random");
if (descriptor.enumerable !== false ||
    descriptor.writable !== true ||
    descriptor.configurable !== true) {
  throw "Math.random should be writable, non-enumerable, and configurable";
}
