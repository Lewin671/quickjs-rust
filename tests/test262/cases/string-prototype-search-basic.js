// Derived from: test/built-ins/String/prototype/search/S15.5.4.12_A1_T1.js
var result = "abc".search(/b/);

if (result !== 1) {
  throw "String.prototype.search should return the first match index";
}
