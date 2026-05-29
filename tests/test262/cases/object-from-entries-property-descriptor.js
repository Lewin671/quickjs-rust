// Derived from: test/built-ins/Object/fromEntries/uses-define-semantics.js
var result = Object.fromEntries([["property", "value"]]);
var descriptor = Object.getOwnPropertyDescriptor(result, "property");
if (descriptor.value !== "value" ||
    descriptor.enumerable !== true ||
    descriptor.writable !== true ||
    descriptor.configurable !== true) {
  throw "Object.fromEntries should create configurable writable enumerable data properties";
}
