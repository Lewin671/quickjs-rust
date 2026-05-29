// Derived from: test/built-ins/Array/prototype/flatMap/thisArg-argument.js
var receiver = { offset: 4 };
var result = [1].flatMap(function(value) {
  return [this.offset + value];
}, receiver);
if (result[0] !== 5) {
  throw "Array.prototype.flatMap should pass thisArg to callback";
}
