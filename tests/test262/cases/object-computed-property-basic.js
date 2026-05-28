// Derived from: test/language/expressions/object/cpn-obj-lit-computed-property-name-from-expression-logical-or.js
var key = "answer";
var object = { [key]: 42 };

if (object.answer !== 42) {
  throw;
}

if (object[key] !== 42) {
  throw;
}
