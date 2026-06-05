// Derived from: test/built-ins/Promise/all/resolve-non-thenable.js
var one = { value: 1 };
var two = { value: 2 };
var promise = Promise.all([one, two]);
if (!(promise instanceof Promise)) {
  throw "Promise.all should return a Promise for non-thenable objects";
}
if (Object.prototype.toString.call(promise) !== "[object Promise]") {
  throw "Promise.all should create Promise instances";
}
