// Derived from: test/built-ins/Error/error-message-tostring-symbol.js
try {
  Error(Symbol("message"));
  throw new Error("expected Symbol message conversion to throw");
} catch (error) {
  if (!(error instanceof TypeError)) {
    throw error;
  }
}
