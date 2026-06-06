// Derived from: test/built-ins/Object/getPrototypeOf/15.2.3.2-1-3.js
if (Object.getPrototypeOf(true) !== Boolean.prototype) {
  throw new Error("expected boolean primitive prototype");
}
