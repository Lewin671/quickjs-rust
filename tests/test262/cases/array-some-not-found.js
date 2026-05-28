// Derived from: test/built-ins/Array/prototype/some/15.4.4.17-8-7.js
var result = [1, 2, 3].some(function(value) {
  return value > 5;
});
if (result !== false) {
  throw "Array.prototype.some should return false when no value matches";
}
