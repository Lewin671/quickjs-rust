// Derived from: test/built-ins/Math/clz32/length.js
if (Math.clz32.length !== 1) {
  throw "expected Math.clz32.length to be 1";
}

if (Math.clz32.propertyIsEnumerable("length")) {
  throw "expected Math.clz32.length to be non-enumerable";
}
