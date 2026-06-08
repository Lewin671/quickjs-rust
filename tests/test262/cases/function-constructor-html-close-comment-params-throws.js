// Derived from: test/annexB/built-ins/Function/createdynfn-no-line-terminator-html-close-comment-params.js
var caught = false;
try {
  Function("-->", "");
} catch (error) {
  caught = error instanceof SyntaxError;
}

if (!caught) { throw; }
