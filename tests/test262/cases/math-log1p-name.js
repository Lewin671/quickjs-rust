// Derived from: test/built-ins/Math/log1p/name.js
var descriptor = Object.getOwnPropertyDescriptor(Math.log1p, "name");
if (descriptor.value !== "log1p") {
  throw "expected Math.log1p.name";
}
if (descriptor.writable || descriptor.enumerable || !descriptor.configurable) {
  throw "expected Math.log1p.name descriptor attributes";
}
