(function() {
  var xs = [1, 2, 3, 4];
  var result = xs.fill(0, -3, -1);
  var ys = [1, 2, 3];
  ys.fill();

  var object = { length: 4 };
  var objectResult = Array.prototype.fill.call(object, "x", 1, 3);

  var value = {};
  var start = Number.MAX_SAFE_INTEGER - 3;
  var large = { length: Number.MAX_SAFE_INTEGER };
  Array.prototype.fill.call(large, value, start, start + 3);

  return (result === xs)
    + ":" + xs.join()
    + ":" + ys.length
    + ":" + ys.join()
    + ":" + (Array.prototype.fill.call(true, 1) instanceof Boolean)
    + ":" + (objectResult === object)
    + ":" + object.hasOwnProperty("0")
    + ":" + object[1]
    + ":" + object[2]
    + ":" + object.hasOwnProperty("3")
    + ":" + (large[start] === value)
    + ":" + (large[start + 1] === value)
    + ":" + (large[start + 2] === value);
})()
