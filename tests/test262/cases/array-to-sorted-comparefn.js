// Derived from: test/built-ins/Array/prototype/toSorted/comparefn-controls-sort.js
var ascending = [4, 3, 2, 1].toSorted(function(left, right) {
  return left - right;
});
var descending = [1, 2, 3, 4].toSorted(function(left, right) {
  return right - left;
});
if (ascending.join() !== "1,2,3,4" || descending.join() !== "4,3,2,1") {
  throw "Array.prototype.toSorted should honor comparefn";
}
