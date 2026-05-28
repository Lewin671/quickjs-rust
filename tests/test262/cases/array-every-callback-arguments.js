// Derived from: test/built-ins/Array/prototype/every/15.4.4.16-7-c-ii-1.js
var seen = "";
[10, 20].every(function(value, index, array) {
  seen = seen + value + ":" + index + ":" + (array[index] === value) + "|";
  return true;
});
if (seen !== "10:0:true|20:1:true|") {
  throw "Array.prototype.every should pass value, index, and array";
}
