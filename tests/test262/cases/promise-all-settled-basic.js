// Derived from: test/built-ins/Promise/allSettled/is-function.js
if (typeof Promise.allSettled !== "function") {
  throw "Promise.allSettled should be a function";
}
if (Promise.allSettled.length !== 1) {
  throw "Promise.allSettled.length should be 1";
}
if (Promise.propertyIsEnumerable("allSettled")) {
  throw "Promise.allSettled should not be enumerable";
}
