// Derived from: test/built-ins/Array/prototype/map/15.4.4.19-8-c-ii-1.js
var seen = "";
var source = [10, 20];
var result = source.map(function(value, index, array) {
  seen = seen + value + ":" + index + ":" + (array === source) + ";";
  return value + index;
});
if (seen !== "10:0:true;20:1:true;" || result[0] !== 10 || result[1] !== 21) {
  throw "Array.prototype.map should call callback with value, index, and array";
}
