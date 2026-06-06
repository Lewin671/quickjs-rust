// Derived from: test/built-ins/Math/log/name.js
var descriptor = Object.getOwnPropertyDescriptor(Math.log, "name");
if (descriptor.value !== "log") {
  throw "expected Math.log.name";
}
if (descriptor.writable || descriptor.enumerable || !descriptor.configurable) {
  throw "expected Math.log.name descriptor attributes";
}
