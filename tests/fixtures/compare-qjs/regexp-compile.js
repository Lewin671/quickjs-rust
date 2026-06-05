(function () {
  var re = /abc/gi;
  re.lastIndex = 7;
  var same = re.compile("def");
  var stringCase = (same === re) + ":" + re.source + ":" + re.flags + ":" +
    re.test("DEF") + ":" + re.lastIndex;

  var pattern = /xyz/i;
  pattern.lastIndex = 9;
  re.compile(pattern);
  var regexpCase = re.source + ":" + re.flags + ":" + re.test("XYZ") + ":" +
    pattern.lastIndex + ":" + re.lastIndex;

  var flagsError = false;
  try {
    re.compile(pattern, "g");
  } catch (error) {
    flagsError = error instanceof TypeError;
  }

  return stringCase + "|" + regexpCase + "|" + flagsError;
})()
