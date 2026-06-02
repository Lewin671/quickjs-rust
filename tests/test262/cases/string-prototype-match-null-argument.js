// Derived from: test/built-ins/String/prototype/match/S15.5.4.10_A1_T5.js
var match = "gnulluna".match(null);
if (match[0] !== "null") {
  throw "expected null regexp argument to match its string form";
}
if (match.index !== 1) {
  throw "expected null regexp argument match to report the found index";
}
