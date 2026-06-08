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

  function invalid(source, flags) {
    re = /test262/gi;
    try {
      re.compile(source, flags);
      return "missing";
    } catch (error) {
      return (error instanceof SyntaxError) + ":" + re.toString() + ":" + re.test("TEsT262");
    }
  }

  var invalidCases = [
    invalid("", "igi"),
    invalid("", "gI"),
    invalid("", "w"),
    invalid("?"),
    invalid(".{2,1}"),
    invalid("{", "u"),
    invalid("\\2", "u"),
  ].join(",");

  re.compile("[\ud834\udf06]", "u");
  var unicodeCase = [
    re.test("\ud834"),
    re.test("\udf06"),
    re.test("\ud834\udf06"),
  ].join(":");

  return stringCase + "|" + regexpCase + "|" + flagsError + "|" + invalidCases + "|" + unicodeCase;
})()
