// Derived from: test/built-ins/Math/f16round/name.js
var descriptor = Object.getOwnPropertyDescriptor(Math.f16round, "name");
if (descriptor.value !== "f16round") {
  throw "expected Math.f16round.name";
}
if (descriptor.writable || descriptor.enumerable || !descriptor.configurable) {
  throw "expected Math.f16round.name descriptor attributes";
}
