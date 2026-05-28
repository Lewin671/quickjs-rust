// Derived from: test/built-ins/Array/prototype/some/15.4.4.17-5-2.js
var receiver = { target: 20 };
var result = [10, 20].some(function(value) {
  return this === receiver && value === this.target;
}, receiver);
if (result !== true) {
  throw "Array.prototype.some should call with thisArg";
}
