// Derived from: test/built-ins/Math/pow/length.js
if (Math.pow.length !== 2) {
  throw "expected Math.pow.length to be 2";
}

if (Math.pow.propertyIsEnumerable("length")) {
  throw "expected Math.pow.length to be non-enumerable";
}
