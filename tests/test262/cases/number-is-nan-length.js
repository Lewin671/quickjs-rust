// Derived from: test/built-ins/Number/isNaN/length.js
if (Number.isNaN.length !== 1) {
  throw "expected Number.isNaN.length to be 1";
}
if (Number.isNaN.propertyIsEnumerable("length")) {
  throw "expected Number.isNaN.length to be non-enumerable";
}
