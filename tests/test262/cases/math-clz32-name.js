// Derived from: test/built-ins/Math/clz32/name.js
var descriptor = Object.getOwnPropertyDescriptor(Math.clz32, "name");
if (descriptor.value !== "clz32") {
  throw "expected Math.clz32.name";
}
if (descriptor.writable || descriptor.enumerable || !descriptor.configurable) {
  throw "expected Math.clz32.name descriptor attributes";
}
