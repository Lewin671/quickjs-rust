// Derived from: test/built-ins/Array/prototype/findIndex/return-index-predicate-result-is-true.js
var index = [11, 12, 13, 14].findIndex(function(value) {
  return value > 12;
});
if (index !== 2) {
  throw "Array.prototype.findIndex should return the first selected index";
}
