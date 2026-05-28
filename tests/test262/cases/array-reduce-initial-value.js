// Derived from: test/built-ins/Array/prototype/reduce/15.4.4.21-9-c-ii-7.js
var result = [1, 2, 3].reduce(function(accumulator, value) {
  return accumulator + value;
}, 10);
if (result !== 16) {
  throw "Array.prototype.reduce should use initialValue";
}
