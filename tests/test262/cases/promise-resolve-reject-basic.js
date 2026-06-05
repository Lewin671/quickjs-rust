// Derived from: test/built-ins/Promise/resolve-non-thenable-immed.js
var resolved = Promise.resolve(1);
var rejected = Promise.reject(2);
if (!(resolved instanceof Promise)) {
  throw "Promise.resolve should create Promise instances";
}
if (!(rejected instanceof Promise)) {
  throw "Promise.reject should create Promise instances";
}
if (Promise.resolve(resolved) !== resolved) {
  throw "Promise.resolve should return promise arguments unchanged";
}
if (Object.prototype.toString.call(resolved) !== "[object Promise]") {
  throw "Resolved promises should have the Promise toString tag";
}
