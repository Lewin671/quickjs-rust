// Derived from: test/built-ins/String/prototype/codePointAt/return-first-code-unit.js
if ("abc".codePointAt(0) !== 97) {
  throw "expected codePointAt(0) to return the first code unit";
}
if ("abc".codePointAt(1) !== 98) {
  throw "expected codePointAt(1) to return the second code unit";
}
if ("abc".codePointAt() !== 97) {
  throw "expected omitted position to select index zero";
}
