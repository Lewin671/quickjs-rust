// Derived from: test/built-ins/Array/prototype/filter/15.4.4.20-9-c-ii-1.js
var seen = "";
var result = [9].filter(function(value, index, array) {
  seen = value + ":" + index + ":" + (array[0] === value);
  return true;
});
if (seen !== "9:0:true" || result[0] !== 9) {
  throw "Array.prototype.filter should pass callback value, index, and array";
}
