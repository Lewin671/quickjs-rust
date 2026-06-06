// Derived from: test/built-ins/Math/acos/name.js
var descriptor = Object.getOwnPropertyDescriptor(Math.acos, "name");
if (descriptor.value !== "acos") {
  throw "expected Math.acos.name";
}
if (descriptor.writable || descriptor.enumerable || !descriptor.configurable) {
  throw "expected Math.acos.name descriptor attributes";
}
