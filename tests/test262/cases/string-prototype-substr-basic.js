// Derived from: test/annexB/built-ins/String/prototype/substr/start-and-length-as-numbers.js
if ("abc".substr(0, 1) !== "a") {
  throw "substr should read the requested number of characters";
}

if ("abc".substr(1, 2) !== "bc") {
  throw "substr should start at the requested character";
}

if ("abc".substr(3, 1) !== "") {
  throw "substr should return empty string when start is at the end";
}
