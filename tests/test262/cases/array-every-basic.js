// Derived from: test/built-ins/Array/prototype/every/15.4.4.16-7-c-ii-5.js
var result = [11, 12, 13, 14].every(function(value) {
  return value > 10;
});
if (result !== true) {
  throw "Array.prototype.every should return true when every value matches";
}
