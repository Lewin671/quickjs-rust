(function () {
  var value = Math.random();
  var descriptor = Object.getOwnPropertyDescriptor(Math, "random");
  return [
    typeof value,
    value >= 0,
    value < 1,
    Math.random.length,
    descriptor.enumerable,
    descriptor.writable,
    descriptor.configurable
  ].join(":");
})()
