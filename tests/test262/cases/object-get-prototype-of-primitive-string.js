// Derived from: test/built-ins/Object/getPrototypeOf/15.2.3.2-1-4.js
if (Object.getPrototypeOf("value") !== String.prototype) {
  throw new Error("expected string primitive prototype");
}
