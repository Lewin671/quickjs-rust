// Derived from: test/built-ins/Object/getPrototypeOf/15.2.3.2-1.js
if (Object.getPrototypeOf(1) !== Number.prototype) {
  throw new Error("expected number primitive prototype");
}
