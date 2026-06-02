// Derived from: test/built-ins/String/prototype/match/S15.5.4.10_A2_T2.js
var matches = "343443444".match(/34/g);
if (matches.length !== 3) {
  throw "expected global match to return all complete matches";
}
if (matches[0] !== "34" || matches[1] !== "34" || matches[2] !== "34") {
  throw "expected global match results to preserve match order";
}
