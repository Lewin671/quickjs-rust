// Derived from: test/built-ins/Array/prototype/flat/length.js
if (Array.prototype.flat.length !== 0) {
  throw "Array.prototype.flat.length should be 0";
}
if (Array.prototype.flat.propertyIsEnumerable("length")) {
  throw "Array.prototype.flat.length should be non-enumerable";
}
