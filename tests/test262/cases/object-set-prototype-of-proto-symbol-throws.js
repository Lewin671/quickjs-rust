// Derived from: test/built-ins/Object/setPrototypeOf/proto-not-obj.js
try {
  Object.setPrototypeOf({}, Symbol("proto"));
  throw new Error("expected Symbol prototype to throw");
} catch (error) {
  if (!(error instanceof TypeError)) {
    throw error;
  }
}
