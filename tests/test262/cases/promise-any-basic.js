// Derived from: test/built-ins/Promise/any/is-function.js
if (typeof Promise.any !== "function") {
  throw "Promise.any should be a function";
}
if (Promise.any.length !== 1) {
  throw "Promise.any.length should be 1";
}
if (Promise.propertyIsEnumerable("any")) {
  throw "Promise.any should not be enumerable";
}
