// Derived from: test/built-ins/Array/prototype/findLast/predicate-call-parameters.js
var seen = "";
[10, 20].findLast(function(value, index, array) {
  seen = seen + value + ":" + index + ":" + (array[index] === value) + "|";
  return false;
});
if (seen !== "20:1:true|10:0:true|") {
  throw "Array.prototype.findLast should pass value, index, and array from right to left";
}
