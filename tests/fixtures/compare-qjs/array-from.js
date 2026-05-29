(function () {
  var source = [0, "foo", undefined, Infinity];
  var copy = Array.from(source);
  var chars = Array.from("Test").join("");
  var objectValues = Array.from({ length: 3, 0: "a", 2: "c" }).join("|");
  var mapped = Array.from([1, 2], function (value, index) {
    return value + index + this.offset;
  }, { offset: 4 }).join();
  return copy.length
    + ":" + copy[0]
    + ":" + copy[1]
    + ":" + (copy[2] === undefined)
    + ":" + copy[3]
    + ":" + (copy === source)
    + ":" + chars
    + ":" + objectValues
    + ":" + mapped
    + ":" + Array.from.length;
})()
