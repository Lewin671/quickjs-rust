// Derived from: test/built-ins/String/prototype/padStart/fill-string-omitted.js
if ("abc".padStart(5) !== "  abc") {
  throw "expected padStart to default fill string to a space";
}
if ("abc".padStart(5, undefined) !== "  abc") {
  throw "expected padStart undefined fill string to default to a space";
}
