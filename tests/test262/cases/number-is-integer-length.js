// Derived from: test/built-ins/Number/isInteger/length.js
if (Number.isInteger.length !== 1) {
  throw "expected Number.isInteger.length to be 1";
}
if (Number.isInteger.propertyIsEnumerable("length")) {
  throw "expected Number.isInteger.length to be non-enumerable";
}
