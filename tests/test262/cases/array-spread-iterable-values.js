// Derived from: test/built-ins/Set/prototype/union/combines-sets.js
// Derived from: test/built-ins/Set/prototype/union/appends-new-values.js
// Derived from: test/built-ins/Set/prototype/union/set-like-array.js
// Derived from: test/language/expressions/array/spread-obj-with-iterator.js
var setValues = [...new Set([1, 2])];
if (setValues.length !== 2 || setValues[0] !== 1 || setValues[1] !== 2) {
  throw "array spread should consume Set iterators";
}

var mapValues = [...new Map([["a", 3]])];
if (mapValues.length !== 1 || mapValues[0][0] !== "a" || mapValues[0][1] !== 3) {
  throw "array spread should consume Map iterators";
}

var customValues = [...{
  [Symbol.iterator]: function() {
    return ["z"][Symbol.iterator]();
  }
}];
if (customValues.length !== 1 || customValues[0] !== "z") {
  throw "array spread should consume custom iterators";
}

var unionValues = [...new Set([1, 2]).union(new Set([2, 3]))];
if (
  unionValues.length !== 3 ||
  unionValues[0] !== 1 ||
  unionValues[1] !== 2 ||
  unionValues[2] !== 3
) {
  throw "array spread should expose Set.prototype.union result values";
}

var setLike = [5, 6];
setLike.size = 3;
setLike.has = function() {
  throw "Set.prototype.union should not call has for appended values";
};
setLike.keys = function() {
  return [2, 3, 4].values();
};
var setLikeUnion = [...new Set([1, 2]).union(setLike)];
if (
  setLikeUnion.length !== 4 ||
  setLikeUnion[0] !== 1 ||
  setLikeUnion[1] !== 2 ||
  setLikeUnion[2] !== 3 ||
  setLikeUnion[3] !== 4
) {
  throw "Set.prototype.union should consume set-like keys through array spread";
}
