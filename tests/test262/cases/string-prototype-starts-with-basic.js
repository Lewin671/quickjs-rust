// Derived from: test/built-ins/String/prototype/startsWith/searchstring-found-without-position.js
if (!"test262".startsWith("test")) {
  throw "expected startsWith to match at the beginning";
}
if ("test262".startsWith("262")) {
  throw "expected startsWith to reject non-prefix search string";
}
