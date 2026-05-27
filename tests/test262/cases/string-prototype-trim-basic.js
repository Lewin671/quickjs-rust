// Derived from: test/built-ins/String/prototype/trim/15.5.4.20-0-1.js
if (typeof String.prototype.trim !== "function") {
  throw "expected trim to exist";
}
if ("  abc  ".trim() !== "abc") {
  throw "expected trim to remove both leading and trailing whitespace";
}
if ("abc".trim() !== "abc") {
  throw "expected trim to preserve strings without edge whitespace";
}
