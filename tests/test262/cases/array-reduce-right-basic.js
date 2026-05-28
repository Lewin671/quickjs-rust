// Derived from: test/built-ins/Array/prototype/reduceRight/15.4.4.22-10-2.js
var result = [1, 2, 3].reduceRight(function(accumulator, value) {
  return accumulator + "-" + value;
});
if (result !== "3-2-1") {
  throw "Array.prototype.reduceRight should accumulate from right to left";
}
