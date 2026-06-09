// Derived from: test/built-ins/RegExp/prototype/Symbol.replace/subst-matched.js

var direct = RegExp.prototype[Symbol.replace].call(/a(.)/g, "a1 a2", "[$1:$&]");
if (direct !== "[1:a1] [2:a2]") {
  throw "expected direct RegExp.prototype[Symbol.replace] substitutions";
}

var calls = [];
var functional = /(\d)/g[Symbol.replace]("a1b2", function(match, digit, position, input) {
  calls.push(match + ":" + digit + ":" + position + ":" + input.length);
  return digit;
});
if (functional !== "a1b2") {
  throw "expected functional RegExp.prototype[Symbol.replace] result";
}
if (calls.join("|") !== "1:1:1:4|2:2:3:4") {
  throw "expected functional replacement arguments";
}

var re = /(?:)/g;
if (re[Symbol.replace]("a", "-") !== "-a-") {
  throw "expected empty global matches to advance";
}
if (re.lastIndex !== 0) {
  throw "expected global replace to leave lastIndex reset";
}
