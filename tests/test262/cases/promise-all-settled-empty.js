// Derived from: test/built-ins/Promise/allSettled/resolves-empty-array.js
var promise = Promise.allSettled([]);
if (!(promise instanceof Promise)) {
  throw "Promise.allSettled([]) should return a Promise";
}
if (Object.prototype.toString.call(promise) !== "[object Promise]") {
  throw "Promise.allSettled([]) should return a Promise object";
}
