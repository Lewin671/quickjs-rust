// Derived from: test/built-ins/Promise/prototype/then/prop-desc.js
var descriptor = Object.getOwnPropertyDescriptor(Promise.prototype, "then");
if (typeof descriptor.value !== "function") {
  throw "Promise.prototype.then descriptor value should be a function";
}
if (descriptor.writable !== true) {
  throw "Promise.prototype.then should be writable";
}
if (descriptor.enumerable !== false) {
  throw "Promise.prototype.then should be non-enumerable";
}
if (descriptor.configurable !== true) {
  throw "Promise.prototype.then should be configurable";
}
