// Derived from: test/built-ins/Promise/withResolvers/result.js
var instance = Promise.withResolvers();
if (typeof Promise.withResolvers !== "function") {
  throw "Promise.withResolvers should be a function";
}
if (Promise.withResolvers.length !== 0) {
  throw "Promise.withResolvers.length should be 0";
}
if (Promise.propertyIsEnumerable("withResolvers")) {
  throw "Promise.withResolvers should not be enumerable";
}
if (typeof instance !== "object" || instance === null) {
  throw "Promise.withResolvers should return an object";
}
