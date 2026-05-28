// Derived from: test/built-ins/Array/prototype/reduce/15.4.4.21-9-c-ii-5.js
var seen = "";
var source = [10, 20];
source.reduce(function(accumulator, value, index, array) {
  seen = seen + accumulator + ":" + value + ":" + index + ":" + (array === source) + "|";
  return accumulator + value;
}, 5);
if (seen !== "5:10:0:true|15:20:1:true|") {
  throw "Array.prototype.reduce should pass accumulator, value, index, and array";
}
