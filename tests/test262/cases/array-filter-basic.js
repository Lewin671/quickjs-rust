// Derived from: test/built-ins/Array/prototype/filter/15.4.4.20-10-3.js
var result = [11, 12, 13, 14].filter(function(value) {
  return value > 12;
});
if (result.length !== 2 || result[0] !== 13 || result[1] !== 14) {
  throw "Array.prototype.filter should return selected values";
}
