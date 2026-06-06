// Derived from: test/built-ins/Reflect/getPrototypeOf/target-is-symbol-throws.js
try {
  Reflect.getPrototypeOf(Symbol("target"));
  throw new Error("expected Symbol target to throw");
} catch (error) {
  if (!(error instanceof TypeError)) {
    throw error;
  }
}
