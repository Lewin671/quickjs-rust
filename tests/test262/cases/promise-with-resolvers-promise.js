// Derived from: test/built-ins/Promise/withResolvers/promise.js
var instance = Promise.withResolvers();
if (!(instance.promise instanceof Promise)) {
  throw "Promise.withResolvers promise should be a Promise";
}
if (instance.promise.constructor !== Promise) {
  throw "Promise.withResolvers promise constructor should be Promise";
}
