// Derived from: test/built-ins/RegExp/prototype/Symbol.match/g-success-return-val.js

var nonGlobal = RegExp.prototype[Symbol.match].call(/a(.)/, "a1 a2");
if (nonGlobal[0] !== "a1" || nonGlobal[1] !== "1" || nonGlobal.index !== 0) {
  throw "expected non-global RegExp.prototype[Symbol.match] to return exec result";
}

var global = RegExp.prototype[Symbol.match].call(/a./g, "a1 a2");
if (global.length !== 2 || global[0] !== "a1" || global[1] !== "a2") {
  throw "expected global RegExp.prototype[Symbol.match] to return matched strings";
}

if (RegExp.prototype[Symbol.match].call(/z/g, "a1 a2") !== null) {
  throw "expected unmatched global RegExp.prototype[Symbol.match] to return null";
}

var empty = /(?:)/g[Symbol.match]("a");
if (empty.length !== 2 || empty[0] !== "" || empty[1] !== "") {
  throw "expected empty global matches to advance";
}

var calls = 0;
var custom = {
  flags: "g",
  lastIndex: 0,
  exec: function() {
    calls += 1;
    if (calls === 1) {
      this.lastIndex = 1;
      return { 0: "x", index: 0, length: 1 };
    }
    return null;
  }
};
var customResult = RegExp.prototype[Symbol.match].call(custom, "abc");
if (customResult.length !== 1 || customResult[0] !== "x" || custom.lastIndex !== 1) {
  throw "expected RegExp.prototype[Symbol.match] to use custom exec";
}
