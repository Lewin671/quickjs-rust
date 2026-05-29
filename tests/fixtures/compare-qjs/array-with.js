(function () {
  var source = [1, 2, 3];
  var changed = source.with(1, 9);
  var negative = source.with(-1, 7);
  var missingValue = source.with(1);
  var object = Array.prototype.with.call({ length: 3, 0: "a", 2: "c" }, 1, "b");
  var string = Array.prototype.with.call("abc", -2, "x");

  return changed.join()
    + ":" + source.join()
    + ":" + (changed === source)
    + ":" + negative.join()
    + ":" + missingValue.join("|")
    + ":" + (missingValue[1] === undefined)
    + ":" + object.join("|")
    + ":" + string.join("")
    + ":" + Array.prototype.with.length;
})()
