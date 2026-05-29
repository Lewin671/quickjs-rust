(function () {
  var basic = [1, [2, 3], 4].flat().join();
  var shallow = [1, [2, [3]]].flat().join("|");
  var deep = [1, [2, [3, [4]]]].flat(Infinity).join();
  var zero = [1, [2]].flat(0).join("|");
  var stringDepth = [1, [2]].flat("1").join();
  var values = [1, [null, undefined]].flat();
  return basic
    + ":" + shallow
    + ":" + deep
    + ":" + zero
    + ":" + stringDepth
    + ":" + values.length
    + ":" + (values[1] === null)
    + ":" + (values[2] === undefined)
    + ":" + Array.prototype.flat.length;
})()
