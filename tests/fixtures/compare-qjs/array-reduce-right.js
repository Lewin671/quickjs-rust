(function () {
  var source = [1, 2, 3];
  var seen = "";
  var joined = source.reduceRight(function (accumulator, value, index, array) {
    seen = seen + accumulator + ":" + value + ":" + index + ":" + (array === source) + "|";
    return accumulator + "-" + value;
  });
  var sum = source.reduceRight(function (accumulator, value) {
    return accumulator + value;
  }, 10);
  var emptyInitial = [].reduceRight(function () {
    return 99;
  }, 7);
  return joined + ":" + sum + ":" + emptyInitial + ":" + seen + ":" + Array.prototype.reduceRight.length;
})()
