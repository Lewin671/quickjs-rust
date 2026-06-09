(function() {
  var counted = /a[a-z]{2,4}?/.exec({ toString: function() { return "abcdefghi"; } });
  var plus = /a+?/.exec("aaa");
  var optional = /a??a/.exec("a");
  return [
    counted[0] + "@" + counted.index,
    plus[0] + "@" + plus.index,
    optional[0] + "@" + optional.index
  ].join("|");
})()
