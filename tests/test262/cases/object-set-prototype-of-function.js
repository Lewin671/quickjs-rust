// Derived from: test/built-ins/Object/setPrototypeOf/success.js
var proto = { marker: 13 };
function target() {}
if (Object.setPrototypeOf(target, proto) !== target) {
  throw new Error("function target should be returned");
}
if (target.marker !== 13) {
  throw new Error("function should inherit from the new prototype");
}
if (Object.getPrototypeOf(target) !== proto) {
  throw new Error("function prototype should be replaced");
}
Object.setPrototypeOf(target, null);
if (Object.getPrototypeOf(target) !== null) {
  throw new Error("function prototype should be null");
}
