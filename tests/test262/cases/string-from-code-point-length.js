// Derived from: test/built-ins/String/fromCodePoint/length.js
if (String.fromCodePoint.length !== 1) {
  throw "expected String.fromCodePoint.length to be 1";
}
if (String.propertyIsEnumerable("fromCodePoint")) {
  throw "expected String.fromCodePoint to be non-enumerable";
}
