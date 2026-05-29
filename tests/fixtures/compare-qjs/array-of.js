(function () {
  var values = Array.of(1, "x", true, null, undefined);
  var single = Array.of(3);
  var empty = Array.of();
  return values.length
    + ":" + values[0]
    + ":" + values[1]
    + ":" + values[2]
    + ":" + (values[3] === null)
    + ":" + (values[4] === undefined)
    + ":" + single.length
    + ":" + single[0]
    + ":" + empty.length
    + ":" + Array.isArray(values)
    + ":" + Array.of.length;
})()
