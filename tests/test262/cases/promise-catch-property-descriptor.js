// Derived from: test/built-ins/Promise/prototype/catch/prop-desc.js
var descriptor = Object.getOwnPropertyDescriptor(Promise.prototype, "catch");
if (typeof descriptor.value !== "function") {
  throw "Promise.prototype.catch descriptor value should be a function";
}
if (descriptor.writable !== true) {
  throw "Promise.prototype.catch should be writable";
}
if (descriptor.enumerable !== false) {
  throw "Promise.prototype.catch should be non-enumerable";
}
if (descriptor.configurable !== true) {
  throw "Promise.prototype.catch should be configurable";
}
