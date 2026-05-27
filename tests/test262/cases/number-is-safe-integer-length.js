// Derived from: test/built-ins/Number/isSafeInteger/length.js
if (Number.isSafeInteger.length !== 1) {
  throw "expected Number.isSafeInteger.length to be 1";
}
if (Number.isSafeInteger.propertyIsEnumerable("length")) {
  throw "expected Number.isSafeInteger.length to be non-enumerable";
}
