(function () {
  var receiver = { limit: 4 };
  var seen = "";
  var source = [1, 2, 3];
  var result = source.every(function (value, index, array) {
    seen = seen + value + ":" + index + ":" + (array === source) + "|";
    return this === receiver && value < this.limit;
  }, receiver);
  var rejected = source.every(function (value) {
    return value < 3;
  });
  var empty = [].every(function () {
    return false;
  });
  return result + ":" + rejected + ":" + empty + ":" + seen + ":" + Array.prototype.every.length;
})()
