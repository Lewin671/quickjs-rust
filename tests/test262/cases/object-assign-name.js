// Derived from: test/built-ins/Object/assign/name.js
var descriptor = Object.getOwnPropertyDescriptor(Object.assign, "name");
if (descriptor.value !== "assign") {
  throw "expected Object.assign.name";
}
if (descriptor.writable || descriptor.enumerable || !descriptor.configurable) {
  throw "expected Object.assign.name descriptor attributes";
}
