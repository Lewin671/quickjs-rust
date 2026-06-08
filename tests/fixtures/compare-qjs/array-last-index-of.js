(function () {
  var target = function () {};
  function values(a, b) {
    arguments[2] = function () {};
    return Array.prototype.lastIndexOf.call(arguments, target)
      + ":" + Array.prototype.lastIndexOf.call(arguments, arguments[2]);
  }
  return [1, 2, 1].lastIndexOf(1)
    + ":" + [1, 2, 1].lastIndexOf(1, 1)
    + ":" + [1, 2, 1].lastIndexOf(1, -1)
    + ":" + [false, "false", false].lastIndexOf(false)
    + ":" + [].lastIndexOf(1)
    + ":" + (function () {} === function () {})
    + ":" + values(0, target);
})()
