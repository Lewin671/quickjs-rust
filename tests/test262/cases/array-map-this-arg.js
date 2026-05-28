// Derived from: test/built-ins/Array/prototype/map/15.4.4.19-5-2.js
var receiver = { offset: 4 };
var result = [1].map(function(value) {
  return this.offset + value;
}, receiver);
if (result[0] !== 5) {
  throw "Array.prototype.map should pass thisArg to callback";
}
