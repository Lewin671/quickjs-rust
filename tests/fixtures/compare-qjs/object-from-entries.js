(function () {
  var entry = { 0: "objectKey", 1: 7 };
  var result = Object.fromEntries([["first", 1], ["first", 2], [3, 4], entry]);
  var descriptor = Object.getOwnPropertyDescriptor(result, "first");
  return [
    Object.fromEntries.length,
    Object.keys(Object.fromEntries([])).length,
    result.first,
    result[3],
    result.objectKey,
    Object.getPrototypeOf(result) === Object.prototype,
    descriptor.enumerable,
    descriptor.writable,
    descriptor.configurable
  ].join(":");
})()
