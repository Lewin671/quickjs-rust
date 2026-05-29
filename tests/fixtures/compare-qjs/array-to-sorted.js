(function () {
  var source = [3, 20, 100, 1];
  var sorted = source.toSorted();
  var numeric = [3, 1, 2].toSorted(function (left, right) {
    return left - right;
  });
  var reverse = [3, 1, 2].toSorted(function (left, right) {
    return right - left;
  });
  var withUndefined = ["b", undefined, "a"].toSorted();
  var object = Array.prototype.toSorted.call({ length: 3, 0: 4, 1: 0, 2: 1 }, function (left, right) {
    return left - right;
  });

  return sorted.join()
    + ":" + source.join()
    + ":" + (sorted === source)
    + ":" + numeric.join()
    + ":" + reverse.join()
    + ":" + withUndefined.join("|")
    + ":" + (withUndefined[2] === undefined)
    + ":" + object.join()
    + ":" + Array.prototype.toSorted.length;
})()
