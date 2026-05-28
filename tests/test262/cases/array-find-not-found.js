// Derived from: test/built-ins/Array/prototype/find/return-undefined-if-predicate-returns-false-value.js
var found = [1, 2, 3].find(function(value) {
  return value > 5;
});
if (found !== undefined) {
  throw "Array.prototype.find should return undefined when no value matches";
}
