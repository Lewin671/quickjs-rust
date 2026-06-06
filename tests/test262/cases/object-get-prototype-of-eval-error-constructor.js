// Derived from: test/built-ins/Object/getPrototypeOf/15.2.3.2-2-12.js
if (Object.getPrototypeOf(EvalError) !== Error) {
  throw new Error("expected EvalError constructor prototype");
}
