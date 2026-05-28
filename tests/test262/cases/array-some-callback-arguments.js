// Derived from: test/built-ins/Array/prototype/some/15.4.4.17-7-c-ii-1.js
var seen = "";
[10, 20].some(function(value, index, array) {
  seen = seen + value + ":" + index + ":" + (array[index] === value) + "|";
  return false;
});
if (seen !== "10:0:true|20:1:true|") {
  throw "Array.prototype.some should pass value, index, and array";
}
