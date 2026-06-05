// Derived from: test/built-ins/Promise/allSettled/resolved-all-fulfilled.js
var promise = Promise.allSettled([Promise.resolve(3)]);
if (!(promise instanceof Promise)) {
  throw "Promise.allSettled should return a Promise for fulfilled inputs";
}
