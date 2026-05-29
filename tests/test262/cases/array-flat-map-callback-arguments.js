// Derived from: test/built-ins/Array/prototype/flatMap/array-like-objects.js
var source = [10, 20];
var seen = "";
var result = source.flatMap(function(value, index, array) {
  seen = seen + value + ":" + index + ":" + (array === source) + ";";
  return [value + index];
});
if (seen !== "10:0:true;20:1:true;" || result[0] !== 10 || result[1] !== 21) {
  throw "Array.prototype.flatMap should call callback with value, index, and array";
}
