// Derived from: test/built-ins/Array/prototype/with/length.js
if (Array.prototype.with.length !== 2) {
  throw "Array.prototype.with.length should be 2";
}
if (Array.prototype.with.propertyIsEnumerable("length")) {
  throw "Array.prototype.with.length should be non-enumerable";
}
