// Derived from: test/built-ins/Array/prototype/toReversed/immutable.js
var source = [1, 2, 3];
var result = source.toReversed();
if (result.join() !== "3,2,1" || source.join() !== "1,2,3" || result === source) {
  throw "Array.prototype.toReversed should return a reversed copy";
}
