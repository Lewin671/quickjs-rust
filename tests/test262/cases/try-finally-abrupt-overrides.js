// Derived from: test/language/statements/try/completion-values-fn-finally-abrupt.js
try {
  try {
    throw "try";
  } finally {
    throw "finally";
  }
} catch (error) {
  if (error !== "finally") { throw; }
}
