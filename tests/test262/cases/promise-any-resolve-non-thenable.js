// Derived from: test/built-ins/Promise/any/resolve-non-thenable.js
var promise = Promise.any([1, Promise.reject(2)]);
if (!(promise instanceof Promise)) {
  throw "Promise.any should return a Promise for non-thenables";
}
