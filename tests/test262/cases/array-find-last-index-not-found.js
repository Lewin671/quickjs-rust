// Derived from: test/built-ins/Array/prototype/findLastIndex/return-negative-one-if-predicate-returns-false-value.js
var index = [11, 12, 13].findLastIndex(function(value) {
  return value > 99;
});
if (index !== -1) {
  throw "Array.prototype.findLastIndex should return -1 when no element matches";
}
