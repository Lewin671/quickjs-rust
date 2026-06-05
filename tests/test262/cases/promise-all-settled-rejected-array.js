// Derived from: test/built-ins/Promise/allSettled/resolved-all-mixed.js
var promise = Promise.allSettled([Promise.reject(3), Promise.resolve(4)]);
if (!(promise instanceof Promise)) {
  throw "Promise.allSettled should return a Promise for mixed settlements";
}
