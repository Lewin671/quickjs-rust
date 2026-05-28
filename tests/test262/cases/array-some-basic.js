// Derived from: test/built-ins/Array/prototype/some/15.4.4.17-7-c-ii-5.js
var result = [11, 12, 13, 14].some(function(value) {
  return value > 12;
});
if (result !== true) {
  throw "Array.prototype.some should return true when a value matches";
}
