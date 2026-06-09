// Derived from: test/built-ins/String/prototype/matchAll/regexp-is-undefined.js
var matches = Array.from("a".matchAll(undefined));
if (matches.length !== 2) {
  throw "expected undefined matchAll pattern to produce empty matches";
}
if (matches[0][0] !== "" || matches[0].index !== 0 || matches[0].input !== "a") {
  throw "expected first empty match at index 0";
}
if (matches[1][0] !== "" || matches[1].index !== 1 || matches[1].input !== "a") {
  throw "expected second empty match at index 1";
}

var regexpMatches = Array.from("a1 a2".matchAll(/a./g));
if (regexpMatches.length !== 2) {
  throw "expected global regexp to produce two matches";
}
if (regexpMatches[0][0] !== "a1" || regexpMatches[0].index !== 0) {
  throw "expected first regexp match";
}
if (regexpMatches[1][0] !== "a2" || regexpMatches[1].index !== 3) {
  throw "expected second regexp match";
}

var threw = false;
try {
  "".matchAll(/a/);
} catch (error) {
  threw = error instanceof TypeError;
}
if (!threw) {
  throw "expected non-global regexp to be rejected";
}
