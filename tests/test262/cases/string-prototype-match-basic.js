// Derived from: test/built-ins/String/prototype/match/S15.5.4.10_A2_T1.js
var match = "1234567890".match(3);
if (match[0] !== "3") {
  throw "expected String.prototype.match to use RegExp-compatible non-RegExp input";
}
if (match.length !== 1) {
  throw "expected String.prototype.match to return a single-element match array";
}
if (match.index !== 2) {
  throw "expected String.prototype.match to preserve match index";
}
if (match.input !== "1234567890") {
  throw "expected String.prototype.match to preserve input";
}
