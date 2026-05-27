// Derived from: test/built-ins/String/prototype/trimStart/this-value-whitespace.js
if ("  abc  ".trimStart() !== "abc  ") {
  throw "expected trimStart to remove leading whitespace";
}
if ("abc".trimStart() !== "abc") {
  throw "expected trimStart to preserve strings without leading whitespace";
}
