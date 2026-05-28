// Derived from: test/built-ins/Array/prototype/findLastIndex/predicate-call-this-non-strict.js
var receiver = { target: 2 };
var index = [1, 2].findLastIndex(function(value) {
  return this === receiver && value === this.target;
}, receiver);
if (index !== 1) {
  throw "Array.prototype.findLastIndex should bind thisArg";
}
