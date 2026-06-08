(function () {
  var minusZero = {
    size: 1,
    has: function () {
      throw new Error("has should not be called");
    },
    keys: function () {
      return [-0].values();
    }
  };
  var first = new Set([1, 2]).symmetricDifference(minusZero);

  var arrayLike = [5];
  arrayLike.size = 3;
  arrayLike.has = function () {
    throw new Error("has should not be called");
  };
  arrayLike.keys = function () {
    return [2, 3, 4].values();
  };
  var second = new Set([1, 2]).symmetricDifference(arrayLike);

  return [first.size, Object.is([...first][2], 0), [...first].join("|"), [...second].join("|")].join(":");
})()
