(function () {
  var re = /a/;
  Object.defineProperty(re, Symbol.match, {
    get: function () {
      re.compile("b");
    },
  });

  var result = re[Symbol.split]("abba");
  return result.length + ":" + result.join("|") + ":" + re.toString();
})()
