// Derived from: test/built-ins/Math/sinh/name.js
var descriptor = Object.getOwnPropertyDescriptor(Math.sinh, "name");
if (descriptor.value !== "sinh") {
  throw "expected Math.sinh.name";
}
if (descriptor.writable || descriptor.enumerable || !descriptor.configurable) {
  throw "expected Math.sinh.name descriptor attributes";
}
