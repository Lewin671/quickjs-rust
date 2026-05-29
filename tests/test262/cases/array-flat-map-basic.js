// Derived from: test/built-ins/Array/prototype/flatMap/array-like-objects-nested.js
var result = [1, 2, 3].flatMap(function(value) {
  return [value, value * 2];
});
if (result.length !== 6 || result[0] !== 1 || result[1] !== 2 || result[4] !== 3 || result[5] !== 6) {
  throw "Array.prototype.flatMap should map values and flatten one level";
}
