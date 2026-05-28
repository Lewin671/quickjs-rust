// Derived from: test/built-ins/Array/prototype/findLastIndex/predicate-call-parameters.js
var source = [10, 20];
var seen = "";
source.findLastIndex(function(value, index, array) {
  seen = seen + value + ":" + index + ":" + (array === source) + "|";
  return false;
});
if (seen !== "20:1:true|10:0:true|") {
  throw "Array.prototype.findLastIndex should pass value, index, and array from right to left";
}
