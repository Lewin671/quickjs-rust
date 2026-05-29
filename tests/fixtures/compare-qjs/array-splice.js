(function () {
  var xs = [1, 2, 3, 4];
  var removed = xs.splice(1, 2, "a", "b", "c");
  var ys = [1, 2, 3];
  var tail = ys.splice(-2);
  var zs = [1, undefined, 3];
  var undef = zs.splice(1, 1, 2);
  return removed.join()
    + ":" + xs.join()
    + ":" + tail.join()
    + ":" + ys.join()
    + ":" + (undef[0] === undefined)
    + ":" + zs.join()
    + ":" + Array.prototype.splice.length;
})()
