// Derived from: test/built-ins/String/prototype/padStart/fill-string-empty.js
if ("abc".padStart(5, "") !== "abc") {
  throw "expected padStart with empty fill string to return the original string";
}
