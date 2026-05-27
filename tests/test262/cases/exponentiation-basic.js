// Derived from: test/language/expressions/exponentiation/exp-operator.js
if ((2 ** 3) !== 8) {
  throw;
}

if ((3 * 2 ** 3) !== 24) {
  throw;
}

if ((2 ** 3 ** 2) !== 512) {
  throw;
}

if ((2 ** -1 * 2) !== 1) {
  throw;
}
