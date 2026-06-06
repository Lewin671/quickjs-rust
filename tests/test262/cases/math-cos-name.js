// Derived from: test/built-ins/Math/cos/name.js
var descriptor = Object.getOwnPropertyDescriptor(Math.cos, "name");
if (descriptor.value !== "cos") {
  throw "expected Math.cos.name";
}
if (descriptor.writable || descriptor.enumerable || !descriptor.configurable) {
  throw "expected Math.cos.name descriptor attributes";
}
