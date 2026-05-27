// Derived from: test/built-ins/Math/sign/length.js
if (Math.sign.length !== 1) {
  throw "expected Math.sign.length to be 1";
}

if (Math.sign.propertyIsEnumerable("length")) {
  throw "expected Math.sign.length to be non-enumerable";
}
