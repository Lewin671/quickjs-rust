// Derived from: test/built-ins/String/prototype/charCodeAt/S15.5.4.5_A2.js
var negative = "abc".charCodeAt(-1);
if (negative === negative) {
  throw "expected negative charCodeAt index to return NaN";
}
