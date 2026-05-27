// Derived from: test/built-ins/Number/isFinite/length.js
if (Number.isFinite.length !== 1) {
  throw "expected Number.isFinite.length to be 1";
}
if (Number.isFinite.propertyIsEnumerable("length")) {
  throw "expected Number.isFinite.length to be non-enumerable";
}
