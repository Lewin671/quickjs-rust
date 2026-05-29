// Derived from: test/built-ins/Array/prototype/toSorted/length-tolength.js
var result = Array.prototype.toSorted.call({ length: "3", 0: 4, 1: 0, 2: 1 }, function(left, right) {
  return left - right;
});
if (result.length !== 3 || result[0] !== 0 || result[1] !== 1 || result[2] !== 4) {
  throw "Array.prototype.toSorted should sort array-like values";
}
