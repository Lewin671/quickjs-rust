// Derived from: test/built-ins/Math/sign/name.js
var descriptor = Object.getOwnPropertyDescriptor(Math.sign, "name");
if (descriptor.value !== "sign") {
  throw "expected Math.sign.name";
}
if (descriptor.writable || descriptor.enumerable || !descriptor.configurable) {
  throw "expected Math.sign.name descriptor attributes";
}
