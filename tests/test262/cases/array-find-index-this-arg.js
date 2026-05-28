// Derived from: test/built-ins/Array/prototype/findIndex/predicate-call-this-non-strict.js
var receiver = { target: 20 };
var index = [10, 20].findIndex(function(value) {
  return this === receiver && value === this.target;
}, receiver);
if (index !== 1) {
  throw "Array.prototype.findIndex should pass thisArg to callback";
}
