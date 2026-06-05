// Derived from: test/built-ins/Promise/try/prop-desc.js
if (typeof Promise.try !== "function") {
  throw "Promise.try should be a function";
}
if (Promise.try.length !== 1) {
  throw "Promise.try.length should be 1";
}
if (Promise.propertyIsEnumerable("try")) {
  throw "Promise.try should not be enumerable";
}
if (!(Promise.try(function() {}) instanceof Promise)) {
  throw "Promise.try should return a Promise";
}
