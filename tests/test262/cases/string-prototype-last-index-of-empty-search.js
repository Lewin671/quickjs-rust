// Derived from: test/built-ins/String/prototype/lastIndexOf/S15.5.4.8_A1_T8.js
if ("abc".lastIndexOf("") !== 3) {
  throw "expected empty search to match at string length";
}
if ("abc".lastIndexOf("", 1) !== 1) {
  throw "expected empty search to honor position";
}
if ("abc".lastIndexOf("", 99) !== 3) {
  throw "expected empty search position to clamp to string length";
}
