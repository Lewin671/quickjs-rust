// Derived from: test/built-ins/Promise/prototype/finally/prop-desc.js
var descriptor = Object.getOwnPropertyDescriptor(Promise.prototype, "finally");
if (typeof descriptor.value !== "function") {
  throw "Promise.prototype.finally descriptor value should be a function";
}
if (descriptor.writable !== true) {
  throw "Promise.prototype.finally should be writable";
}
if (descriptor.enumerable !== false) {
  throw "Promise.prototype.finally should be non-enumerable";
}
if (descriptor.configurable !== true) {
  throw "Promise.prototype.finally should be configurable";
}
