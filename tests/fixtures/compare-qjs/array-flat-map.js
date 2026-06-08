(function () {
  var doubled = [1, 2, 3].flatMap(function (value) {
    return [value, value * 2];
  }).join();
  var shallow = [1, 2].flatMap(function (value) {
    return [[value]];
  });
  var source = [10, 20];
  var seen = "";
  var mapped = source.flatMap(function (value, index, array) {
    seen = seen + value + ":" + index + ":" + (array === source) + ":" + this.offset + ";";
    return [value + index + this.offset];
  }, { offset: 3 });
  var called = 0;
  var booleanFlatMap = Array.prototype.flatMap.call(true, function () {
    called = called + 1;
  });
  var sparseCalls = [];
  var sparse = [1, , 3].flatMap(function (value, index) {
    sparseCalls.push(index);
    return [value];
  });
  return doubled
    + ":" + shallow.length
    + ":" + Array.isArray(shallow[0])
    + ":" + shallow[0][0]
    + ":" + seen
    + ":" + mapped.join()
    + ":" + booleanFlatMap.length
    + ":" + called
    + ":" + Array.prototype.flatMap.length
    + ":" + sparse.join()
    + ":" + sparseCalls.join();
})()
