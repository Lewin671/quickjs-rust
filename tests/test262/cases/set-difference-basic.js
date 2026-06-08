// Derived from: test/built-ins/Set/prototype/difference/combines-sets.js
// Derived from: test/built-ins/Set/prototype/difference/combines-Map.js
// Derived from: test/built-ins/Set/prototype/difference/allows-set-like-object.js
// Derived from: test/built-ins/Set/prototype/difference/converts-negative-zero.js

var result = new Set([1, 2]).difference(new Set([2, 3]));
var seen = "";
result.forEach(function(value) { seen = seen + value; });
if (result.size !== 1 || seen !== "1") {
  throw "difference should keep left-only values";
}

var mapResult = new Set([1, 2]).difference(new Map([[2, "two"], [3, "three"]]));
var mapSeen = "";
mapResult.forEach(function(value) { mapSeen = mapSeen + value; });
if (mapResult.size !== 1 || mapSeen !== "1") {
  throw "difference should accept Map set-like arguments";
}

var setLike = {
  size: 2,
  has: function(value) {
    return value === 2;
  },
  keys: function() {
    return [2, 3].values();
  }
};
var setLikeResult = new Set([1, 2]).difference(setLike);
var setLikeSeen = "";
setLikeResult.forEach(function(value) { setLikeSeen = setLikeSeen + value; });
if (setLikeResult.size !== 1 || setLikeSeen !== "1") {
  throw "difference should accept set-like object arguments";
}

var minusZeroLike = {
  size: 1,
  has: function() {
    throw "has should not be called";
  },
  keys: function() {
    return [-0].values();
  }
};
var minusZeroResult = new Set([0, 1]).difference(minusZeroLike);
var minusZeroSeen = "";
minusZeroResult.forEach(function(value) { minusZeroSeen = minusZeroSeen + value; });
if (minusZeroResult.size !== 1 || minusZeroSeen !== "1") {
  throw "difference should normalize set-like -0 keys";
}
