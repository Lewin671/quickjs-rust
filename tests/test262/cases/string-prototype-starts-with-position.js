// Derived from: test/built-ins/String/prototype/startsWith/searchstring-found-with-position.js
if (!"test262".startsWith("262", 4)) {
  throw "expected startsWith to honor position";
}
if ("test262".startsWith("262", 5)) {
  throw "expected startsWith to reject mismatched position";
}
