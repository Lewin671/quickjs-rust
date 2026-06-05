// Derived from: test/built-ins/Promise/allSettled/resolve-non-thenable.js
var promise = Promise.allSettled([{ value: 1 }, 2]);
if (!(promise instanceof Promise)) {
  throw "Promise.allSettled should return a Promise for non-thenables";
}
