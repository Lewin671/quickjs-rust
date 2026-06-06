// Derived from: test/built-ins/Object/getPrototypeOf/15.2.3.2-2-15.js
if (Object.getPrototypeOf(SyntaxError) !== Error) {
  throw new Error("expected SyntaxError constructor prototype");
}
