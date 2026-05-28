// Derived from: test/built-ins/Array/prototype/every/15.4.4.16-8-1.js
var result = [].every(function() {
  return false;
});
if (result !== true) {
  throw "Array.prototype.every should return true for an empty array";
}
