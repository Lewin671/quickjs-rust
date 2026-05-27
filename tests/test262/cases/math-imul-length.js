// Derived from: test/built-ins/Math/imul/length.js
if (Math.imul.length !== 2) {
  throw "expected Math.imul.length to be 2";
}

if (Math.imul.propertyIsEnumerable("length")) {
  throw "expected Math.imul.length to be non-enumerable";
}
