// Derived from: test/built-ins/Array/prototype/find/predicate-call-this-non-strict.js
var receiver = { target: 20 };
var found = [10, 20].find(function(value) {
  return this === receiver && value === this.target;
}, receiver);
if (found !== 20) {
  throw "Array.prototype.find should call with thisArg";
}
