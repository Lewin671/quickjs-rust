// Derived from: test/built-ins/RegExp/15.10.2.15-6-1.js
var caught = false;

try {
  new RegExp("^[z-a]$");
} catch (error) {
  caught = error instanceof SyntaxError;
}

if (!caught) {
  throw "RegExp should reject descending character class ranges";
}
