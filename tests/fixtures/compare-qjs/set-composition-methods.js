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
  return values(left.union(right)) + ":" +
    values(left.intersection(right)) + ":" +
    values(left.difference(right)) + ":" +
    values(left.symmetricDifference(right)) + ":" +
    left.isSubsetOf(new Set([1, 2, 3])) + ":" +
    left.isSubsetOf(right) + ":" +
    left.isSupersetOf(new Set([1])) + ":" +
    left.isSupersetOf(right) + ":" +
    left.isDisjointFrom(new Set([3])) + ":" +
    left.isDisjointFrom(right);
})()
