// Derived from: test/built-ins/Array/prototype/find/return-found-value-predicate-result-is-true.js
var found = [11, 12, 13, 14].find(function(value) {
  return value > 12;
});
if (found !== 13) {
  throw "Array.prototype.find should return the first selected value";
}
