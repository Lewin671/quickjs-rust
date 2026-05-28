// Derived from: test/built-ins/String/prototype/lastIndexOf/S15.5.4.8_A1_T1.js
if ("abcabc".lastIndexOf("bc") !== 4) {
  throw "expected lastIndexOf to find the last substring";
}
if ("abcabc".lastIndexOf("z") !== -1) {
  throw "expected lastIndexOf to return -1 when missing";
}
