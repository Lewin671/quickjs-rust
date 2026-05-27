// Derived from: test/built-ins/String/prototype/endsWith/searchstring-found-with-position.js
if (!"test262".endsWith("test", 4)) {
  throw "expected endsWith to honor end position";
}
if ("test262".endsWith("262", 5)) {
  throw "expected endsWith to reject mismatched end position";
}
