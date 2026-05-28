// Derived from: test/built-ins/Array/prototype/findLast/return-found-value-predicate-result-is-true.js
var found = [11, 12, 13, 14].findLast(function(value) {
  return value > 12;
});
if (found !== 14) {
  throw "Array.prototype.findLast should return the last selected value";
}
