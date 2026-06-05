// Derived from: test/built-ins/Promise/prototype/catch/S25.4.5.1_A1.1_T1.js
if (typeof Promise.prototype.catch !== "function") {
  throw "Promise.prototype.catch should be a function";
}
if (Promise.prototype.catch.length !== 1) {
  throw "Promise.prototype.catch.length should be 1";
}
var promise = Promise.resolve(1);
if (typeof promise.catch !== "function") {
  throw "Promise instances should inherit catch";
}
var next = promise.catch(function() {});
if (!(next instanceof Promise)) {
  throw "Promise.prototype.catch should return a Promise";
}
if (next === promise) {
  throw "Promise.prototype.catch should return a distinct Promise";
}
