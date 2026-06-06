// Derived from: test/built-ins/Math/acosh/name.js
var descriptor = Object.getOwnPropertyDescriptor(Math.acosh, "name");
if (descriptor.value !== "acosh") {
  throw "expected Math.acosh.name";
}
if (descriptor.writable || descriptor.enumerable || !descriptor.configurable) {
  throw "expected Math.acosh.name descriptor attributes";
}
