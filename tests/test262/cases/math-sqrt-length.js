// Derived from: test/built-ins/Math/sqrt/length.js
if (Math.sqrt.length !== 1) {
  throw "expected Math.sqrt.length to be 1";
}

if (Math.sqrt.propertyIsEnumerable("length")) {
  throw "expected Math.sqrt.length to be non-enumerable";
}
