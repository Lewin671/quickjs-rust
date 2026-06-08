// Derived from: test/built-ins/Array/prototype/splice/call-with-boolean.js
var truthy = Array.prototype.splice.call(true);
if (truthy.length !== 0) {
  throw "Array.prototype.splice.call(true) should return an empty array";
}

var falsy = Array.prototype.splice.call(false);
if (falsy.length !== 0) {
  throw "Array.prototype.splice.call(false) should return an empty array";
}
