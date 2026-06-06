(function () {
  var entry = { 0: "objectKey", 1: 7 };
  var symbol = Symbol();
  var result = Object.fromEntries([["first", 1], ["first", 2], [3, 4], entry, [symbol, 11]]);
  var descriptor = Object.getOwnPropertyDescriptor(result, "first");
  return [
    Object.fromEntries.length,
    Object.keys(Object.fromEntries([])).length,
    result.first,
    result[3],
    result.objectKey,
    result[symbol],
    Object.getPrototypeOf(result) === Object.prototype,
    descriptor.enumerable,
    descriptor.writable,
    descriptor.configurable
  ].join(":");
})()
