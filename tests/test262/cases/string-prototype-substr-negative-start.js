// Derived from: test/annexB/built-ins/String/prototype/substr/start-negative.js
if ("abc".substr(-1) !== "c") {
  throw "substr should resolve negative start from the end";
}

if ("abc".substr(-4) !== "abc") {
  throw "substr should clamp negative start before the beginning";
}

if ("abc".substr(-1.1) !== "c") {
  throw "substr should truncate floating negative start toward zero";
}
