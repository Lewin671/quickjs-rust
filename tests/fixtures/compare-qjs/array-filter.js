(function() {
  var receiver = { threshold: 2 };
  var source = [1, 2, 3, 4];
  var result = source.filter(function(value, index, array) {
    return this === receiver && array[index] === value && value > this.threshold;
  }, receiver);
  return result.join("|") + ":" + source.join("|") + ":" + (result !== source) + ":" + Array.prototype.filter.length;
})()
