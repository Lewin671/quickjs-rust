(function () {
  return "abcdef".substr(1, 3)
    + ":" + "abcdef".substr(-2)
    + ":" + "abcdef".substr(-20, 2)
    + ":" + "abcdef".substr(2, -1)
    + ":" + "abcdef".substr(2, 2.8)
    + ":" + "abcdef".substr(Infinity, 1)
    + ":" + "abc".substr(1, undefined)
    + ":" + String.prototype.substr.length;
})()
