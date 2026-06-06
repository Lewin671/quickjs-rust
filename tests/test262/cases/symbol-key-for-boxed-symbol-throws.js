// Derived from: test/built-ins/Symbol/keyFor/arg-non-symbol.js
try {
  Symbol.keyFor(Object(Symbol("s")));
  throw new Error("expected boxed Symbol to throw");
} catch (error) {
  if (!(error instanceof TypeError)) {
    throw error;
  }
}
