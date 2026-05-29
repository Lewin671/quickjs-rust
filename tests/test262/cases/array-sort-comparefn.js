// Derived from: test/built-ins/Array/prototype/sort/S15.4.4.11_A1.2_T1.js
var array = [3, 1, 2];
array.sort(function(left, right) {
  return left - right;
});
if (array.join() !== "1,2,3") {
  throw "Array.prototype.sort should use comparefn ordering";
}
