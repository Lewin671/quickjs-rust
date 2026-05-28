(function () {
  var receiver = { target: 3 };
  var seen = "";
  var source = [1, 2, 3, 4];
  var found = source.find(function (value, index, array) {
    seen = seen + value + ":" + index + ":" + (array === source) + "|";
    return this === receiver && value === this.target;
  }, receiver);
  var missing = source.find(function (value) {
    return value > 9;
  });
  return found + ":" + missing + ":" + seen + ":" + Array.prototype.find.length;
})()
