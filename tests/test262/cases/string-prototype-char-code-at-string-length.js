// Derived from: test/built-ins/String/prototype/charCodeAt/S15.5.4.5_A3.js
var atLength = "abc".charCodeAt(3);
if (atLength === atLength) {
  throw "expected charCodeAt at string length to return NaN";
}
