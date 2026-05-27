// Derived from: test/built-ins/Math/abs/length.js
if (Math.abs.length !== 1) {
  throw "expected Math.abs.length to be 1";
}

if (Math.abs.propertyIsEnumerable("length")) {
  throw "expected Math.abs.length to be non-enumerable";
}
