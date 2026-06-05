// Derived from: test/built-ins/Promise/withResolvers/resolvers.js
var instance = Promise.withResolvers();
if (typeof instance.resolve !== "function") {
  throw "resolve should be a function";
}
if (instance.resolve.length !== 1 || instance.resolve.name !== "") {
  throw "resolve should be an unnamed unary function";
}
if (typeof instance.reject !== "function") {
  throw "reject should be a function";
}
if (instance.reject.length !== 1 || instance.reject.name !== "") {
  throw "reject should be an unnamed unary function";
}
