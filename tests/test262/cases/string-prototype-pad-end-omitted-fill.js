// Derived from: test/built-ins/String/prototype/padEnd/fill-string-omitted.js
if ("abc".padEnd(5) !== "abc  ") {
  throw "expected padEnd to default fill string to a space";
}
if ("abc".padEnd(5, undefined) !== "abc  ") {
  throw "expected padEnd undefined fill string to default to a space";
}
