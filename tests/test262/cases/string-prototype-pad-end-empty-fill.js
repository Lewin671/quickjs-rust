// Derived from: test/built-ins/String/prototype/padEnd/fill-string-empty.js
if ("abc".padEnd(5, "") !== "abc") {
  throw "expected padEnd with empty fill string to return the original string";
}
