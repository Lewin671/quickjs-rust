// Derived from: test/built-ins/Promise/constructor.js
if (typeof Promise !== "function") {
  throw "Promise should be a function";
}
if (Promise.length !== 1) {
  throw "Promise.length should be 1";
}
var called = "";
var promise = new Promise(function(resolve, reject) {
  called = typeof resolve + ":" + typeof reject;
  resolve(1);
});
if (!(promise instanceof Promise)) {
  throw "new Promise should create Promise instances";
}
if (Object.prototype.toString.call(promise) !== "[object Promise]") {
  throw "Promise instances should have the Promise toString tag";
}
if (called !== "function:function") {
  throw "Promise executor should receive resolving functions";
}
