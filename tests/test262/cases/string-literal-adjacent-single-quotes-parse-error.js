// Derived from: test/language/types/string/S8.4_A13_T3.js
var caught = false;
try {
  eval("var str = '''';");
} catch (error) {
  caught = error instanceof SyntaxError;
}
if (!caught) {
  throw new Test262Error("adjacent single-quoted strings without a separator must be a SyntaxError");
}
