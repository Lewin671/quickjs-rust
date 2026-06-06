// Derived from: test/built-ins/Math/fround/name.js
var descriptor = Object.getOwnPropertyDescriptor(Math.fround, "name");
if (descriptor.value !== "fround") {
  throw "expected Math.fround.name";
}
if (descriptor.writable || descriptor.enumerable || !descriptor.configurable) {
  throw "expected Math.fround.name descriptor attributes";
}
