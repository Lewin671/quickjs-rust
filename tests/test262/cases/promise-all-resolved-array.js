// Derived from: test/built-ins/Promise/all/S25.4.4.1_A7.1_T1.js
var promise = Promise.all([Promise.resolve(3)]);
if (!(promise instanceof Promise)) {
  throw "Promise.all should return a Promise for resolved promise inputs";
}
if (Object.prototype.toString.call(promise) !== "[object Promise]") {
  throw "Promise.all should create Promise instances";
}
