// Derived from: test/built-ins/Array/prototype/reduce/15.4.4.21-10-2.js
var result = [11, 12, 13].reduce(function(accumulator, value) {
  return accumulator + value;
});
if (result !== 36) {
  throw "Array.prototype.reduce should accumulate values without initialValue";
}
