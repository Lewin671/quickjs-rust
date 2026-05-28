// Derived from: test/language/expressions/object/prop-def-id-get-error.js
var answer = 42;
var object = { answer };

if (object.answer !== 42) {
  throw;
}
