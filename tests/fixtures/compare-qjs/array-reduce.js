(function () {
  var source = [1, 2, 3];
  var seen = "";
  var sum = source.reduce(function (accumulator, value, index, array) {
    seen = seen + accumulator + ":" + value + ":" + index + ":" + (array === source) + "|";
    return accumulator + value;
  }, 10);
  var noInitial = source.reduce(function (accumulator, value) {
    return accumulator + value;
  });
  var emptyInitial = [].reduce(function () {
    return 99;
  }, 7);
  return sum + ":" + noInitial + ":" + emptyInitial + ":" + seen + ":" + Array.prototype.reduce.length;
})()
