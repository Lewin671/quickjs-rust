(function() {
  var re = /./ug;
  var match = re.exec("\uD834\uDF06");
  var index = /a/u.exec("\uD834\uDF06a").index;
  return match.index + ":" + match[0].length + ":" + re.lastIndex + ":" + index;
})()
