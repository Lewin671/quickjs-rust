// Derived from: test/built-ins/Promise/any/resolved-sequence.js
var promise = Promise.any([Promise.reject(1), Promise.resolve(2)]);
if (!(promise instanceof Promise)) {
  throw "Promise.any should return a Promise for fulfilled inputs";
}
