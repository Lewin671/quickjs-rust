// Derived from: test/built-ins/Object/getPrototypeOf/15.2.3.2-2-17.js
if (Object.getPrototypeOf(URIError) !== Error) {
  throw new Error("expected URIError constructor prototype");
}
