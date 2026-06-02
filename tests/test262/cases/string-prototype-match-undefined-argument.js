// Derived from: test/built-ins/String/prototype/match/S15.5.4.10_A1_T7.js
var match = "undefined".match(undefined);
if (match.length !== 1) {
  throw "expected undefined regexp argument to produce one empty match";
}
if (match[0] !== "") {
  throw "expected undefined regexp argument to use RegExp(undefined)";
}
if (match.index !== 0 || match.input !== "undefined") {
  throw "expected undefined regexp argument match metadata";
}
