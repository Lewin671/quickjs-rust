// Derived from: test/language/statements/class/class-name-ident-static.js
// Derived from: test/language/statements/class/class-name-ident-static-escaped.js
// Derived from: test/language/statements/class/class-name-ident-let-escaped.js
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

expectSyntaxError("class static {}");
expectSyntaxError("class st\\u0061tic {}");
expectSyntaxError("class l\\u0065t {}");
