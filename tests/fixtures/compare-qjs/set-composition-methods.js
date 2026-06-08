(function () {
  function values(set) {
    var result = "";
    set.forEach(function (value) {
      result = result + value;
    });
    return result;
  }

  var left = new Set([1, 2]);
  var right = new Set([2, 3]);
  var setLike = {
    size: 2,
    has: function (value) {
      return value === 2;
    },
    keys: function () {
      return [2, 3].values();
    }
  };
  var minusZeroLike = {
    size: 1,
    has: function () {
      throw new Error("has should not be called");
    },
    keys: function () {
      return [-0].values();
    }
  };
  return values(left.union(right)) + ":" +
    values(left.intersection(right)) + ":" +
    values(left.difference(right)) + ":" +
    values(left.symmetricDifference(right)) + ":" +
    left.isSubsetOf(new Set([1, 2, 3])) + ":" +
    left.isSubsetOf(right) + ":" +
    left.isSupersetOf(new Set([1])) + ":" +
    left.isSupersetOf(right) + ":" +
    left.isDisjointFrom(new Set([3])) + ":" +
    left.isDisjointFrom(right) + ":" +
    values(left.difference(new Map([[2, "two"], [3, "three"]]))) + ":" +
    values(left.difference(setLike)) + ":" +
    values(new Set([0, 1]).difference(minusZeroLike)) + ":" +
    new Set([1, 2, 3]).isSupersetOf(setLike) + ":" +
    new Set([1]).isDisjointFrom(setLike);
})()
