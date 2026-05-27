// Derived from: test/built-ins/Math/max/length.js
if (Math.max.length !== 2) {
  throw "expected Math.max.length to be 2";
}

if (Math.max.propertyIsEnumerable("length")) {
  throw "expected Math.max.length to be non-enumerable";
}
