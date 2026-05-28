(function () {
  var receiver = { total: 0 };
  var seen = "";
  var source = [1, 2, 3];
  var result = source.forEach(function (value, index, array) {
    this.total = this.total + value;
    seen = seen + value + ":" + index + ":" + (array === source) + "|";
    return 99;
  }, receiver);
  return seen + ":" + receiver.total + ":" + result + ":" + Array.prototype.forEach.length;
})()
