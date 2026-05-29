// Derived from: test/annexB/built-ins/String/prototype/substr/length-negative.js
if ("abc".substr(1, 0) !== "") {
  throw "substr should return empty string for zero length";
}

if ("abc".substr(1, -1) !== "") {
  throw "substr should return empty string for negative length";
}
