// Derived from: test/built-ins/Math/round/name.js
var descriptor = Object.getOwnPropertyDescriptor(Math.round, "name");
if (descriptor.value !== "round") {
  throw "expected Math.round.name";
}
if (descriptor.writable || descriptor.enumerable || !descriptor.configurable) {
  throw "expected Math.round.name descriptor attributes";
}
