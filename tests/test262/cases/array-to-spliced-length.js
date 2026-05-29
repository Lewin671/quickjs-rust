// Derived from: test/built-ins/Array/prototype/toSpliced/length.js
if (Array.prototype.toSpliced.length !== 2) {
  throw "Array.prototype.toSpliced.length should be 2";
}
if (Array.prototype.toSpliced.propertyIsEnumerable("length")) {
  throw "Array.prototype.toSpliced.length should be non-enumerable";
}
