// Derived from: test/built-ins/Reflect/setPrototypeOf/proto-is-symbol-throws.js
try {
  Reflect.setPrototypeOf({}, Symbol("proto"));
  throw new Error("expected Symbol prototype to throw");
} catch (error) {
  if (!(error instanceof TypeError)) {
    throw error;
  }
}
