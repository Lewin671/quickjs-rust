// Derived from: test/language/expressions/exponentiation/exp-assignment-operator.js
var base = -3;

if ((base **= 3) !== -27) {
  throw;
}

if (base !== -27) {
  throw;
}
