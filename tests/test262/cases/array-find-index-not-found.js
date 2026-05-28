// Derived from: test/built-ins/Array/prototype/findIndex/return-negative-one-if-predicate-returns-false-value.js
var index = [11, 12, 13].findIndex(function(value) {
  return value > 99;
});
if (index !== -1) {
  throw "Array.prototype.findIndex should return -1 when no element is selected";
}
