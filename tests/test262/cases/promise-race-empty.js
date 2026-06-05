// Derived from: test/built-ins/Promise/race/S25.4.4.3_A2.1_T1.js
var promise = Promise.race([]);
if (!(promise instanceof Promise)) {
  throw "Promise.race should return a Promise";
}
if (Object.prototype.toString.call(promise) !== "[object Promise]") {
  throw "Promise.race should create Promise instances";
}
