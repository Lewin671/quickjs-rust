(function() {
  var direct = RegExp.prototype[Symbol.match].call(/a(.)/, "a1 a2")[0];
  var global = RegExp.prototype[Symbol.match].call(/a./g, "a1 a2").join("|");
  var none = RegExp.prototype[Symbol.match].call(/z/g, "a1 a2");
  var empty = /(?:)/g[Symbol.match]("a").join("|");
  var custom = {
    flags: "g",
    lastIndex: 0,
    exec: function() {
      if (this.lastIndex === 0) {
        this.lastIndex = 1;
        return { 0: "x", index: 0, length: 1 };
      }
      return null;
    }
  };
  return direct + ":" + global + ":" + none + ":" + empty + ":" +
    RegExp.prototype[Symbol.match].call(custom, "abc").join("|") + ":" +
    custom.lastIndex;
})()
