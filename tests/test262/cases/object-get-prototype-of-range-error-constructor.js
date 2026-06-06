// Derived from: test/built-ins/Object/getPrototypeOf/15.2.3.2-2-13.js
if (Object.getPrototypeOf(RangeError) !== Error) {
  throw new Error("expected RangeError constructor prototype");
}
