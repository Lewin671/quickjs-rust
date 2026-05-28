// Derived from: test/built-ins/Array/prototype/reduceRight/15.4.4.22-9-c-ii-5.js
var seen = "";
var source = [10, 20];
source.reduceRight(function(accumulator, value, index, array) {
  seen = seen + accumulator + ":" + value + ":" + index + ":" + (array === source) + "|";
  return accumulator + value;
}, 5);
if (seen !== "5:20:1:true|25:10:0:true|") {
  throw "Array.prototype.reduceRight should pass accumulator, value, index, and array";
}
