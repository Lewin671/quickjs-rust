// Derived from: test/built-ins/Array/prototype/every/15.4.4.16-5-2.js
var receiver = { limit: 30 };
var result = [10, 20].every(function(value) {
  return this === receiver && value < this.limit;
}, receiver);
if (result !== true) {
  throw "Array.prototype.every should call with thisArg";
}
