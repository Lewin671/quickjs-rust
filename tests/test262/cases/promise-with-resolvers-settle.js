// Derived from: test/built-ins/Promise/withResolvers/promise.js
var instance = Promise.withResolvers();
var chained = instance.promise.then(function() {});
instance.resolve(1);
if (!(chained instanceof Promise)) {
  throw "Promise.withResolvers resolve should leave a Promise chain";
}
