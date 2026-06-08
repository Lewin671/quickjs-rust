// Derived from: test/built-ins/Set/prototype/symmetricDifference/converts-negative-zero.js
// Derived from: test/built-ins/Set/prototype/symmetricDifference/set-like-array.js

const setlikeWithMinusZero = {
  size: 1,
  has: function () {
    throw "Set.prototype.symmetricDifference should not invoke .has on its argument";
  },
  keys: function () {
    return [-0].values();
  },
};

const combinedWithZero = new Set([1, 2]).symmetricDifference(setlikeWithMinusZero);
const zeroValues = [...combinedWithZero];

if (zeroValues.length !== 3) {
  throw "symmetricDifference should append the set-like -0 key";
}
if (zeroValues[0] !== 1 || zeroValues[1] !== 2 || !Object.is(zeroValues[2], 0)) {
  throw "symmetricDifference should normalize set-like -0 keys to +0";
}
if (!(combinedWithZero instanceof Set)) {
  throw "symmetricDifference should return a Set";
}

const setlikeArray = [5];
setlikeArray.size = 3;
setlikeArray.has = function () {
  throw "Set.prototype.symmetricDifference should not invoke .has on its argument";
};
setlikeArray.keys = function () {
  return [2, 3, 4].values();
};

const combinedWithArray = [...new Set([1, 2]).symmetricDifference(setlikeArray)];

if (combinedWithArray.length !== 3) {
  throw "symmetricDifference should use set-like array keys";
}
if (combinedWithArray[0] !== 1 || combinedWithArray[1] !== 3 || combinedWithArray[2] !== 4) {
  throw "symmetricDifference should consume arrays as set-like objects when set-like methods exist";
}
