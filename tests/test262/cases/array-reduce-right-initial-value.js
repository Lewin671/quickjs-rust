// Derived from: test/built-ins/Array/prototype/reduceRight/15.4.4.22-9-c-ii-7.js
var result = [1, 2, 3].reduceRight(function(accumulator, value) {
  return accumulator + value;
}, 10);
if (result !== 16) {
  throw "Array.prototype.reduceRight should use initialValue";
}
