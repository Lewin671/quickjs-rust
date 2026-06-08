(function () {
  var obj = { constructor: RegExp };
  obj[Symbol.match] = true;
  var same = RegExp(obj) === obj;

  var regexpLike = { source: "source text", flags: "i" };
  regexpLike[Symbol.match] = [];
  var result = new RegExp(regexpLike);

  var override = { source: "override text" };
  Object.defineProperty(override, "flags", {
    get: function () {
      throw "flags";
    },
  });
  override[Symbol.match] = true;
  var overridden = new RegExp(override, "g");

  return same + "|" + result.source + ":" + result.flags + "|" +
    overridden.source + ":" + overridden.flags;
})()
