// Derived from: test/built-ins/Math/floor/name.js
var descriptor = Object.getOwnPropertyDescriptor(Math.floor, "name");
if (descriptor.value !== "floor") {
  throw "expected Math.floor.name";
}
if (descriptor.writable || descriptor.enumerable || !descriptor.configurable) {
  throw "expected Math.floor.name descriptor attributes";
}
