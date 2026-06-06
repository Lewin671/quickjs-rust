// Derived from: test/built-ins/Math/random/name.js
var descriptor = Object.getOwnPropertyDescriptor(Math.random, "name");
if (descriptor.value !== "random") {
  throw "expected Math.random.name";
}
if (descriptor.writable || descriptor.enumerable || !descriptor.configurable) {
  throw "expected Math.random.name descriptor attributes";
}
