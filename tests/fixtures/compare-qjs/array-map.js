(function () {
  let source = [1, 2, 3];
  let receiver = { offset: 4 };
  let mapped = source.map(function (value, index, array) {
    return value + index + this.offset + (array === source ? 10 : 0);
  }, receiver);
  return mapped.join("|") + ":" + source.join("|") + ":" + (mapped !== source) + ":" + Array.prototype.map.length;
})()
