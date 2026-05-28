(function () {
  var receiver = { target: 3 };
  var seen = "";
  var source = [1, 2, 3, 4];
  var result = source.some(function (value, index, array) {
    seen = seen + value + ":" + index + ":" + (array === source) + "|";
    return this === receiver && value === this.target;
  }, receiver);
  var missing = source.some(function (value) {
    return value > 9;
  });
  return result + ":" + missing + ":" + seen + ":" + Array.prototype.some.length;
})()
