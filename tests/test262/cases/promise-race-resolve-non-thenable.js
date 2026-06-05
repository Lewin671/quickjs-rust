// Derived from: test/built-ins/Promise/race/resolve-non-thenable.js
var one = { value: 1 };
var promise = Promise.race([one]);
if (!(promise instanceof Promise)) {
  throw "Promise.race should return a Promise for non-thenable objects";
}
if (Object.prototype.toString.call(promise) !== "[object Promise]") {
  throw "Promise.race should create Promise instances";
}
