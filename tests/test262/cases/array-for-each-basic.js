// Derived from: test/built-ins/Array/prototype/forEach/15.4.4.18-7-c-ii-18.js
var total = 0;
[1, 2, 3].forEach(function(value) {
  total = total + value;
});
if (total !== 6) {
  throw "Array.prototype.forEach should visit each value";
}
