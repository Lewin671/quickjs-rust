// Derived from: test/language/eval-code/direct/var-env-global-lex-non-strict.js
let evalGlobalLexCollision;
var caught = false;
try {
  eval("var evalGlobalLexCollision;");
} catch (error) {
  caught = error instanceof SyntaxError;
}
if (!caught) {
  throw new Test262Error("direct eval var declaration must collide with a global lexical binding");
}
