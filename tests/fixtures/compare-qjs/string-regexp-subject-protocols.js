(function() {
  var log = [];
  var needle = { toString: function() { log.push("needle"); return "b"; } };
  var position = { valueOf: function() { log.push("position"); return 0; } };
  var found = "abc".indexOf(needle, position);
  var re = /b/g;
  Object.defineProperty(re, "flags", { get: function() { log.push("flags"); return "g"; } });
  var input = { toString: function() { log.push("input"); return "abcb"; } };
  var matched = RegExp.prototype[Symbol.match].call(re, input).length;
  function regexpLike(steps) {
    return {
      flags: "g", li: 0, n: 0,
      get lastIndex() { steps.push("get" + this.li); return this.li; },
      set lastIndex(v) { steps.push("set" + v); this.li = v; },
      exec: function(v) { steps.push("exec"); return this.n++ < 2 ? { 0: "", index: this.li, length: 1 } : null; }
    };
  }
  var matchSteps = [];
  var emptyMatches = RegExp.prototype[Symbol.match].call(regexpLike(matchSteps), "aaa").length;
  var replaceSteps = [];
  var emptyReplaced = RegExp.prototype[Symbol.replace].call(regexpLike(replaceSteps), "aaa", "-");
  var gets = 0;
  var counted = /b/g;
  Object.defineProperty(counted, "exec", { get: function() {
    gets++;
    return function(v) { return RegExp.prototype.exec.call(this, v); };
  } });
  var lookups = [ "abab".match(counted).length, "abab".replace(counted, "x"), "abab".search(counted), gets ].join(",");
  var boxed = new String("xyxy");
  var all = [];
  for (var m of boxed.matchAll(/y/g)) all.push(m.index);
  var inputs = [];
  "a,b".replaceAll(",", function(m, p, s) { inputs.push(s); return ";"; });
  var subst = ["x-y-z".replaceAll("-", "[$`|$&|$']"), "x-y-z".replace("-", "<$'>")].join(" ");
  var cursor = 0, fields = 0, doc = "f,".repeat(50);
  while ((cursor = doc.indexOf(",", cursor)) !== -1) { cursor++; fields++; }
  return [found, matched, log.join(","), emptyMatches, matchSteps.join(","), emptyReplaced,
    replaceSteps.join(","), lookups, all.join(","), inputs.join(","), subst, fields,
    "abc".startsWith("c", 2), "abc".endsWith("a", 1), "abc".includes("a", 1),
    boxed.toString() === boxed.valueOf()].join("|");
})()
