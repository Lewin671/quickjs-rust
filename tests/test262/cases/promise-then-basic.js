// Derived from: test/built-ins/Promise/prototype/then/S25.4.5.3_A1.1_T1.js
if (typeof Promise.prototype.then !== "function") {
  throw "Promise.prototype.then should be a function";
}
if (Promise.prototype.then.length !== 2) {
  throw "Promise.prototype.then.length should be 2";
}
var promise = Promise.resolve(1);
if (typeof promise.then !== "function") {
  throw "Promise instances should inherit then";
}
var next = promise.then();
if (!(next instanceof Promise)) {
  throw "Promise.prototype.then should return a Promise";
}
if (next === promise) {
  throw "Promise.prototype.then should return a distinct Promise";
}
