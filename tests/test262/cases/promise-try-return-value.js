// Derived from: test/built-ins/Promise/try/return-value.js
var promise = Promise.try(function() {
  return 5;
});
promise.then(function(value) {
  if (value !== 5) {
    throw "Promise.try should fulfill with callback return value";
  }
});
