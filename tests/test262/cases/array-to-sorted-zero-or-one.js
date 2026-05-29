// Derived from: test/built-ins/Array/prototype/toSorted/zero-or-one-element.js
var zero = [];
var zeroResult = zero.toSorted();
var one = [7];
var oneResult = one.toSorted();
if (zeroResult.length !== 0 || zeroResult === zero || oneResult[0] !== 7 || oneResult === one) {
  throw "Array.prototype.toSorted should return new arrays for zero and one element inputs";
}
