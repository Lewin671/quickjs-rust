// Derived from: test/built-ins/Reflect/setPrototypeOf/target-is-symbol-throws.js
try {
  Reflect.setPrototypeOf(Symbol("target"), null);
  throw new Error("expected Symbol target to throw");
} catch (error) {
  if (!(error instanceof TypeError)) {
    throw error;
  }
}
