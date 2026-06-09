(function() {
  var first = /(aa|aabaac|ba|b|c)*/.exec({
    toString: function() { return {}; },
    valueOf: function() { return "aabaac"; }
  });
  var second = /(z)((a+)?(b+)?(c))*/.exec((function() { return "zaacbbbcac"; })());
  return [
    first[0] + "@" + first.index,
    first[1],
    second[0] + "@" + second.index,
    second[1],
    second[2],
    second[3],
    String(second[4]),
    second[5]
  ].join("|");
})()
