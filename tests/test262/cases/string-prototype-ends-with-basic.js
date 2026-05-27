// Derived from: test/built-ins/String/prototype/endsWith/searchstring-found-without-position.js
if (!"test262".endsWith("262")) {
  throw "expected endsWith to match at the end";
}
if ("test262".endsWith("test")) {
  throw "expected endsWith to reject non-suffix search string";
}
