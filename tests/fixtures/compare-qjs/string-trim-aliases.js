(function () {
  var left = "  abc  ".trimLeft();
  var right = "  abc  ".trimRight();
  var aliases = [
    String.prototype.trimLeft === String.prototype.trimStart,
    String.prototype.trimRight === String.prototype.trimEnd,
    String.prototype.trimLeft.length,
    String.prototype.trimRight.length,
  ].join("|");
  return left + ":" + right + ":" + aliases;
})()
