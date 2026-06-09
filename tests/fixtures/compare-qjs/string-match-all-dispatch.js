(function() {
  var matcher = {
    count: 0
  };
  matcher[Symbol.matchAll] = function(input) {
    this.count = this.count + 1;
    return input + ":" + (this === matcher) + ":" + this.count;
  };
  return "abc".matchAll(matcher) + ":" + String.prototype.matchAll.length;
})()
