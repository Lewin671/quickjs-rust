// Derived from: test/built-ins/Array/prototype/reduce/15.4.4.21-9-1.js
var result = [].reduce(function() {
  throw "callback should not be called for empty array with initialValue";
}, 7);
if (result !== 7) {
  throw "Array.prototype.reduce should return initialValue for an empty array";
}
