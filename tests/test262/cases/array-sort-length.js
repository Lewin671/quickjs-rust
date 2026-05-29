// Derived from: test/built-ins/Array/prototype/sort/length.js
if (Array.prototype.sort.length !== 1) {
  throw "Array.prototype.sort.length should be 1";
}
if (Array.prototype.sort.propertyIsEnumerable("length")) {
  throw "Array.prototype.sort.length should be non-enumerable";
}
