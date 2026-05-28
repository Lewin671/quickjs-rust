// Derived from: test/built-ins/Array/prototype/findLast/predicate-call-this-non-strict.js
var receiver = { target: 10 };
var found = [10, 20].findLast(function(value) {
  return this === receiver && value === this.target;
}, receiver);
if (found !== 10) {
  throw "Array.prototype.findLast should pass thisArg to callback";
}
