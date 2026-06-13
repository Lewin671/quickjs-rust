// Derived from: test/language/statements/class/static-init-await-binding-invalid.js
// Derived from: test/language/statements/class/static-init-invalid-arguments.js
// Derived from: test/language/statements/class/static-init-invalid-await.js
// Derived from: test/language/statements/class/static-init-invalid-return.js
// Derived from: test/language/statements/class/static-init-invalid-yield.js
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

expectSyntaxError("class C { static { class await {} } }");
expectSyntaxError("class C { static { arguments; } }");
expectSyntaxError("class C { static { await; } }");
expectSyntaxError("class C { static { return; } }");
expectSyntaxError("class C { static { yield; } }");
