// Derived from: test/built-ins/Object/fromEntries/empty-iterable.js
var result = Object.fromEntries([]);
if (Object.keys(result).length !== 0 ||
    Object.getPrototypeOf(result) !== Object.prototype) {
  throw "Object.fromEntries should return an empty ordinary object";
}
