// Derived from: test/built-ins/Array/prototype/forEach/15.4.4.18-5-1.js
var receiver = { total: 0 };
[1, 2].forEach(function(value) {
  this.total = this.total + value;
}, receiver);
if (receiver.total !== 3) {
  throw "Array.prototype.forEach should call with thisArg";
}
