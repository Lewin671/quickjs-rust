// Derived from: test/built-ins/Object/getPrototypeOf/15.2.3.2-2-16.js
if (Object.getPrototypeOf(TypeError) !== Error) {
  throw new Error("expected TypeError constructor prototype");
}
