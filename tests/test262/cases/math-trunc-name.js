// Derived from: test/built-ins/Math/trunc/name.js
var descriptor = Object.getOwnPropertyDescriptor(Math.trunc, "name");
if (descriptor.value !== "trunc") {
  throw "expected Math.trunc.name";
}
if (descriptor.writable || descriptor.enumerable || !descriptor.configurable) {
  throw "expected Math.trunc.name descriptor attributes";
}
