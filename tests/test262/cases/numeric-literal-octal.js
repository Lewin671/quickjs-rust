// Derived from: test/language/literals/numeric/octal.js
if (0o0 !== 0) {
  throw "expected 0o0 to be 0";
}
if (0O77 !== 63) {
  throw "expected 0O77 to be 63";
}
if (0o010 !== 8) {
  throw "expected octal literal with leading zero to evaluate";
}
