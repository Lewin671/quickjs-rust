// Derived from: test/built-ins/String/prototype/trimEnd/this-value-whitespace.js
if ("  abc  ".trimEnd() !== "  abc") {
  throw "expected trimEnd to remove trailing whitespace";
}
if ("abc".trimEnd() !== "abc") {
  throw "expected trimEnd to preserve strings without trailing whitespace";
}
