// Derived from: test/built-ins/Array/prototype/forEach/15.4.4.18-7-c-ii-18.js
var seen = "";
[10, 20].forEach(function(value, index, array) {
  seen = seen + value + ":" + index + ":" + (array[index] === value) + "|";
});
if (seen !== "10:0:true|20:1:true|") {
  throw "Array.prototype.forEach should pass value, index, and array";
}
