// Derived from: test/built-ins/Array/prototype/splice/length.js
if (Array.prototype.splice.length !== 2) {
  throw "Array.prototype.splice.length should be 2";
}
if (Array.prototype.splice.propertyIsEnumerable("length")) {
  throw "Array.prototype.splice.length should be non-enumerable";
}
