// Derived from: test/built-ins/Object/getPrototypeOf/15.2.3.2-2-14.js
if (Object.getPrototypeOf(ReferenceError) !== Error) {
  throw new Error("expected ReferenceError constructor prototype");
}
