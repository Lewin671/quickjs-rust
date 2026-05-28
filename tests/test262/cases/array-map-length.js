// Derived from: test/built-ins/Array/prototype/map/length.js
if (Array.prototype.map.length !== 1) {
  throw "Array.prototype.map.length should be 1";
}
if (Array.prototype.map.propertyIsEnumerable("length")) {
  throw "Array.prototype.map.length should be non-enumerable";
}
