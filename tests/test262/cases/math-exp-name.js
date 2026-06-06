// Derived from: test/built-ins/Math/exp/name.js
var descriptor = Object.getOwnPropertyDescriptor(Math.exp, "name");
if (descriptor.value !== "exp") {
  throw "expected Math.exp.name";
}
if (descriptor.writable || descriptor.enumerable || !descriptor.configurable) {
  throw "expected Math.exp.name descriptor attributes";
}
