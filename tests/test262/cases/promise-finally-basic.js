// Derived from: test/built-ins/Promise/prototype/finally/is-a-function.js
if (typeof Promise.prototype.finally !== "function") {
  throw "Promise.prototype.finally should be a function";
}
if (Promise.prototype.finally.length !== 1) {
  throw "Promise.prototype.finally.length should be 1";
}
var promise = Promise.resolve(1);
if (typeof promise.finally !== "function") {
  throw "Promise instances should inherit finally";
}
var next = promise.finally(function() {});
if (!(next instanceof Promise)) {
  throw "Promise.prototype.finally should return a Promise";
}
if (next === promise) {
  throw "Promise.prototype.finally should return a distinct Promise";
}
