(function () {
  var source = [1, 2, 3];
  var reversed = source.toReversed();
  var empty = [].toReversed();
  var single = [7].toReversed();
  var object = Array.prototype.toReversed.call({ length: 3, 0: "a", 2: "c" });
  var string = Array.prototype.toReversed.call("abc");

  return reversed.join()
    + ":" + source.join()
    + ":" + (reversed === source)
    + ":" + empty.length
    + ":" + single[0]
    + ":" + object.join("|")
    + ":" + string.join("")
    + ":" + Array.prototype.toReversed.length;
})()
