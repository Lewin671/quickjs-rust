// Derived from: test/built-ins/Array/prototype/findLast/return-undefined-if-predicate-returns-false-value.js
var found = [11, 12, 13].findLast(function(value) {
  return value > 99;
});
if (found !== undefined) {
  throw "Array.prototype.findLast should return undefined when no element is selected";
}
