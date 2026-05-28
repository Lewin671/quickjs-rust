// Derived from: test/built-ins/Array/prototype/filter/15.4.4.20-5-2.js
var receiver = { keep: 2 };
var result = [1, 2, 3].filter(function(value) {
  return value === this.keep;
}, receiver);
if (result.length !== 1 || result[0] !== 2) {
  throw "Array.prototype.filter should bind thisArg";
}
