(function () {
  var numeric = [3, 1, 2];
  var numericResult = numeric.sort(function (left, right) {
    return left - right;
  });
  var defaultOrder = [3, 20, 100, 1].sort().join();
  var withUndefined = ["b", undefined, "a"];
  var undefinedResult = withUndefined.sort();
  return (numericResult === numeric)
    + ":" + numeric.join()
    + ":" + defaultOrder
    + ":" + withUndefined.join("|")
    + ":" + (withUndefined[2] === undefined)
    + ":" + (undefinedResult === withUndefined)
    + ":" + Array.prototype.sort.length;
})()
