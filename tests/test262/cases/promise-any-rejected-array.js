// Derived from: test/built-ins/Promise/any/reject-all-mixed.js
var promise = Promise.any([Promise.reject(1), Promise.reject(2)]);
if (!(promise instanceof Promise)) {
  throw "Promise.any should return a Promise for rejected inputs";
}
