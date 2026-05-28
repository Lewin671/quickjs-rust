// Derived from: test/built-ins/Array/prototype/find/predicate-call-parameters.js
var seen = "";
[10, 20].find(function(value, index, array) {
  seen = seen + value + ":" + index + ":" + (array[index] === value) + "|";
  return false;
});
if (seen !== "10:0:true|20:1:true|") {
  throw "Array.prototype.find should pass value, index, and array";
}
