// Derived from: test/built-ins/Array/prototype/reduceRight/15.4.4.22-9-1.js
var result = [].reduceRight(function() {
  throw "callback should not be called for empty array with initialValue";
}, 7);
if (result !== 7) {
  throw "Array.prototype.reduceRight should return initialValue for an empty array";
}
