// Derived from: test/language/literals/numeric/binary.js
if (0b0 !== 0) {
  throw "expected 0b0 to be 0";
}
if (0B11 !== 3) {
  throw "expected 0B11 to be 3";
}
if (0b010 !== 2) {
  throw "expected binary literal with leading zero to evaluate";
}
