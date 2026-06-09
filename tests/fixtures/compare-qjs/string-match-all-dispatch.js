(function() {
  var matcher = {
    count: 0
  };
  matcher[Symbol.matchAll] = function(input) {
    this.count = this.count + 1;
    return input + ":" + (this === matcher) + ":" + this.count;
  };
  var custom = "abc".matchAll(matcher) + ":" + String.prototype.matchAll.length;
  var regexp = Array.from("a1 a2".matchAll(/a./g)).map(function(match) {
    return match[0] + "@" + match.index;
  }).join("|");
  var empty = Array.from("a".matchAll()).map(function(match) {
    return match.index;
  }).join(",");
  return custom + ":" + regexp + ":" + empty;
})()
