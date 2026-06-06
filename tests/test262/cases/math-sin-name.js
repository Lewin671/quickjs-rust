// Derived from: test/built-ins/Math/sin/name.js
var descriptor = Object.getOwnPropertyDescriptor(Math.sin, "name");
if (descriptor.value !== "sin") {
  throw "expected Math.sin.name";
}
if (descriptor.writable || descriptor.enumerable || !descriptor.configurable) {
  throw "expected Math.sin.name descriptor attributes";
}
