// Derived from: test/built-ins/Symbol/toStringTag/prop-desc.js
var descriptor = Object.getOwnPropertyDescriptor(Symbol, "toStringTag");

if (typeof Symbol.toStringTag !== "symbol") {
  throw "expected Symbol.toStringTag to be a symbol";
}
if (descriptor.writable !== false) {
  throw "expected Symbol.toStringTag to be non-writable";
}
if (descriptor.enumerable !== false) {
  throw "expected Symbol.toStringTag to be non-enumerable";
}
if (descriptor.configurable !== false) {
  throw "expected Symbol.toStringTag to be non-configurable";
}
