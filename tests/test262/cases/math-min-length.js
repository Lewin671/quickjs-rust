// Derived from: test/built-ins/Math/min/length.js
if (Math.min.length !== 2) {
  throw "expected Math.min.length to be 2";
}

if (Math.min.propertyIsEnumerable("length")) {
  throw "expected Math.min.length to be non-enumerable";
}
