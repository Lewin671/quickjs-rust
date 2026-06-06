// Derived from: test/built-ins/Math/sqrt/name.js
var descriptor = Object.getOwnPropertyDescriptor(Math.sqrt, "name");
if (descriptor.value !== "sqrt") {
  throw "expected Math.sqrt.name";
}
if (descriptor.writable || descriptor.enumerable || !descriptor.configurable) {
  throw "expected Math.sqrt.name descriptor attributes";
}
