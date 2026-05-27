// Derived from: test/built-ins/Array/prototype/at/length.js
if (Array.prototype.at.length !== 1) {
  throw "expected Array.prototype.at.length to be 1";
}

if (Array.prototype.at.propertyIsEnumerable("length")) {
  throw "expected Array.prototype.at.length to be non-enumerable";
}
