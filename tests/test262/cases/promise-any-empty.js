// Derived from: test/built-ins/Promise/any/returns-promise.js
var promise = Promise.any([]);
if (!(promise instanceof Promise)) {
  throw "Promise.any([]) should return a Promise";
}
if (Object.prototype.toString.call(promise) !== "[object Promise]") {
  throw "Promise.any([]) should return a Promise object";
}
