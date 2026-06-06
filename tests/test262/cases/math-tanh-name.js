// Derived from: test/built-ins/Math/tanh/name.js
var descriptor = Object.getOwnPropertyDescriptor(Math.tanh, "name");
if (descriptor.value !== "tanh") {
  throw "expected Math.tanh.name";
}
if (descriptor.writable || descriptor.enumerable || !descriptor.configurable) {
  throw "expected Math.tanh.name descriptor attributes";
}
