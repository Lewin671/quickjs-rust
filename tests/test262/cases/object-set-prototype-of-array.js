// Derived from: test/built-ins/Object/setPrototypeOf/success.js
var proto = { marker: 11 };
var array = [];
if (Object.setPrototypeOf(array, proto) !== array) {
  throw new Error("array target should be returned");
}
if (array.marker !== 11) {
  throw new Error("array should inherit from the new prototype");
}
if (Object.getPrototypeOf(array) !== proto) {
  throw new Error("array prototype should be replaced");
}
Object.setPrototypeOf(array, null);
if (Object.getPrototypeOf(array) !== null) {
  throw new Error("array prototype should be null");
}
