// Derived from: test/built-ins/String/prototype/split/separator-empty-string-instance-is-string.js
var split = "abc".split("");
if (split.length !== 3 || split[0] !== "a" || split[1] !== "b" || split[2] !== "c") {
  throw "expected empty separator to split into characters";
}
