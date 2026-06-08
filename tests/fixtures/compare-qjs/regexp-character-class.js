(function () {
  var result = /[\d][\12-\14]{1,}[^\d]/.exec("line1\n\n\n\n\nline2");
  return result.length + ":" + result.index + ":" + result[0].length + ":" + result[0][0] + ":" + result[0][6];
})()
