// Derived from: test/built-ins/String/prototype/repeat/repeat-string-n-times.js
if ("abc".repeat(3) !== "abcabcabc") {
  throw "expected repeat to copy the string";
}
if ("abc".repeat(0) !== "") {
  throw "expected repeat(0) to return an empty string";
}
if ("abc".repeat(2.8) !== "abcabc") {
  throw "expected repeat to truncate fractional count";
}
