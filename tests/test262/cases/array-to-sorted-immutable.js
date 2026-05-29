// Derived from: test/built-ins/Array/prototype/toSorted/immutable.js
var source = [3, 1, 2];
var result = source.toSorted();
if (result.join() !== "1,2,3" || source.join() !== "3,1,2" || result === source) {
  throw "Array.prototype.toSorted should return a sorted copy";
}
