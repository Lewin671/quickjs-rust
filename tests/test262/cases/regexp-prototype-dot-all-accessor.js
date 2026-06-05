// Derived from: test/built-ins/RegExp/prototype/dotAll/this-val-regexp.js
// Derived from: test/built-ins/RegExp/prototype/dotAll/this-val-regexp-prototype.js
// Derived from: test/built-ins/RegExp/prototype/dotAll/prop-desc.js
var descriptor = Object.getOwnPropertyDescriptor(RegExp.prototype, "dotAll");
if (typeof descriptor.get !== "function") {
  throw "expected RegExp.prototype.dotAll getter";
}
if (descriptor.set !== undefined) {
  throw "expected RegExp.prototype.dotAll setter to be undefined";
}
if (descriptor.enumerable || !descriptor.configurable) {
  throw "expected RegExp.prototype.dotAll descriptor";
}
if (/a/s.dotAll !== true) {
  throw "expected dotAll getter to report s flag";
}
if (/a/.dotAll !== false) {
  throw "expected dotAll getter to reject missing s flag";
}
if (RegExp.prototype.dotAll !== undefined) {
  throw "expected RegExp.prototype dotAll getter to return undefined";
}
var rejected = false;
try {
  descriptor.get.call({});
} catch (error) {
  rejected = error instanceof TypeError;
}
if (!rejected) {
  throw "expected dotAll getter to reject ordinary objects";
}
