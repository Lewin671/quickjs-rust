// Derived from: test/built-ins/Array/prototype/map/15.4.4.19-8-c-iii-1.js
var result = [0, 1, 2, 3, 4].map(function(value) {
  if (value % 2) {
    return 2 * value + 1;
  }
  return value / 2;
});
if (result.length !== 5 || result[0] !== 0 || result[1] !== 3 || result[4] !== 2) {
  throw "Array.prototype.map should return mapped values in a new array";
}
