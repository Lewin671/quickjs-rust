// Derived from: test/built-ins/Math/atanh/name.js
var descriptor = Object.getOwnPropertyDescriptor(Math.atanh, "name");
if (descriptor.value !== "atanh") {
  throw "expected Math.atanh.name";
}
if (descriptor.writable || descriptor.enumerable || !descriptor.configurable) {
  throw "expected Math.atanh.name descriptor attributes";
}
