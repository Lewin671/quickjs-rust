// Derived from: test/built-ins/Array/prototype/findLastIndex/return-index-predicate-result-is-true.js
var index = [11, 12, 13, 14].findLastIndex(function(value) {
  return value > 12;
});
if (index !== 3) {
  throw "Array.prototype.findLastIndex should return the last selected index";
}
