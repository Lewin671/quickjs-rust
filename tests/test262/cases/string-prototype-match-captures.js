// Derived from: test/built-ins/String/prototype/match/S15.5.4.10_A2_T6.js
var string = "Boston, Mass. 02134";
var match = string.match(/([\d]{5})([-\ ]?[\d]{4})?$/);
if (match.length !== 3) {
  throw "expected full match plus two captures";
}
if (match[0] !== "02134" || match[1] !== "02134" || match[2] !== void 0) {
  throw "expected optional capture to be undefined when it does not participate";
}
if (match.index !== 14 || match.input !== string) {
  throw "expected capture match metadata";
}
