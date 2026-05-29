// Derived from: test/built-ins/Array/prototype/toReversed/zero-or-one-element.js
var zero = [];
var zeroResult = zero.toReversed();
var one = [7];
var oneResult = one.toReversed();
if (zeroResult.length !== 0 || zeroResult === zero || oneResult[0] !== 7 || oneResult === one) {
  throw "Array.prototype.toReversed should return new arrays for zero and one element inputs";
}
