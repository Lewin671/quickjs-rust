// Derived from: test/built-ins/String/prototype/search/S15.5.4.12_A2_T3.js
var result = new String("test string").search(/String/i);

if (result !== 5) {
  throw "String.prototype.search should honor RegExp ignore-case matching";
}
