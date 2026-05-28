// Derived from: test/built-ins/String/prototype/lastIndexOf/S15.5.4.8_A4_T3.js
if ("abcabc".lastIndexOf("bc", 3) !== 1) {
  throw "expected lastIndexOf to search backward from position";
}
if ("abcabc".lastIndexOf("bc", 4) !== 4) {
  throw "expected lastIndexOf to include the position index";
}
if ("abcabc".lastIndexOf("bc", -1) !== -1) {
  throw "expected negative position to clamp to zero";
}
