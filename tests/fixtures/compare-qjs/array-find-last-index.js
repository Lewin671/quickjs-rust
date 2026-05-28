(function () {
  var receiver = { target: 2 };
  var seen = "";
  var source = [1, 2, 3, 2];
  var found = source.findLastIndex(function (value, index, array) {
    seen = seen + value + ":" + index + ":" + (array === source) + "|";
    return this === receiver && value === this.target;
  }, receiver);
  var missing = source.findLastIndex(function (value) {
    return value > 9;
  });
  return found + ":" + missing + ":" + seen + ":" + Array.prototype.findLastIndex.length;
})()
