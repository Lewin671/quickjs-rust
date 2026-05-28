(function () {
  var receiver = { target: 2 };
  var seen = "";
  var source = [1, 2, 3, 2];
  var found = source.findLast(function (value, index, array) {
    seen = seen + value + ":" + index + ":" + (array === source) + "|";
    return this === receiver && value === this.target;
  }, receiver);
  var missing = source.findLast(function (value) {
    return value > 9;
  });
  return found + ":" + missing + ":" + seen + ":" + Array.prototype.findLast.length;
})()
