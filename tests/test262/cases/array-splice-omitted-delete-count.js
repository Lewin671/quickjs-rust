// Derived from: test/built-ins/Array/prototype/splice/called_with_one_argument.js
var array = [1, 2, 3];
var removed = array.splice(1);
if (removed.join() !== "2,3") {
  throw "Array.prototype.splice should delete through the end when deleteCount is omitted";
}
if (array.join() !== "1") {
  throw "Array.prototype.splice should keep elements before start";
}
