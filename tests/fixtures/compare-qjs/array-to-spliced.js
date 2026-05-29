(function () {
  var source = [1, 2, 3, 4];
  var replaced = source.toSpliced(1, 2, "a", "b");
  var negative = [1, 2, 3].toSpliced(-1, 1, 9);
  var missingDeleteCount = [1, 2, 3].toSpliced(1);
  var undefinedDeleteCount = [1, 2, 3].toSpliced(1, undefined, 9);
  var startPastLength = [1, 2, 3].toSpliced(8, 1, 4);
  var object = Array.prototype.toSpliced.call({ length: 3, 0: "a", 2: "c" }, 1, 1, "b");
  var string = Array.prototype.toSpliced.call("abc", 1, 1, "x");

  return replaced.join()
    + ":" + source.join()
    + ":" + (replaced === source)
    + ":" + negative.join()
    + ":" + missingDeleteCount.join()
    + ":" + undefinedDeleteCount.join()
    + ":" + startPastLength.join()
    + ":" + object.join("|")
    + ":" + string.join("")
    + ":" + Array.prototype.toSpliced.length;
})()
