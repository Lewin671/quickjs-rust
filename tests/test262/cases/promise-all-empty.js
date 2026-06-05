// Derived from: test/built-ins/Promise/all/S25.4.4.1_A2.1_T1.js
var promise = Promise.all([]);
if (!(promise instanceof Promise)) {
  throw "Promise.all should return a Promise";
}
if (Object.prototype.toString.call(promise) !== "[object Promise]") {
  throw "Promise.all should create Promise instances";
}
