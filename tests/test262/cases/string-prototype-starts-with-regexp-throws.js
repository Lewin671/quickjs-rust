// Derived from: test/built-ins/String/prototype/startsWith/searchstring-is-regexp-throws.js
var caught = false;

try {
  "".startsWith(/./);
} catch (error) {
  caught = error instanceof TypeError;
}

if (!caught) {
  throw "String.prototype.startsWith should reject RegExp search strings";
}
