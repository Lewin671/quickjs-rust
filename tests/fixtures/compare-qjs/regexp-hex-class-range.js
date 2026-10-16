(function () {
  var results = [
    /[\x3a-\x40]/.test(":"),
    /[\x2a-\x2f]/.test("*"),
    /[\x3A-\x40]/.test(":"),
    /[\u003a-\u0040]/.test(":"),
    /[\x3a-\x40]/u.test(":"),
    /[\x3a-\x40]/v.test(":"),
    /[a-\x62]/.test("b"),
    /[\x61-b]/.test("a")
  ];
  var descending;
  try {
    new RegExp("[\\x40-\\x30]");
    descending = "accepted";
  } catch (error) {
    descending = error instanceof SyntaxError;
  }
  return results.join(",") + ":" + descending;
})()
