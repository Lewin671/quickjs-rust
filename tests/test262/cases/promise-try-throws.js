// Derived from: test/built-ins/Promise/try/throws.js
var promise = Promise.try(function() {
  throw 7;
});
promise.catch(function(reason) {
  if (reason !== 7) {
    throw "Promise.try should reject with thrown value";
  }
});
