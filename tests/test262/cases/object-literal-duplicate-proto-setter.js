// Derived from: test/language/expressions/object/__proto__-duplicate.js
function expectSyntaxError(source) {
  var threw = false;
  try {
    Function(source);
  } catch (error) {
    threw = true;
    if (!(error instanceof SyntaxError)) {
      throw error;
    }
  }
  if (!threw) {
    throw new Error("expected SyntaxError");
  }
}

expectSyntaxError("return ({ __proto__: null, other: null, '__proto__': null });");
