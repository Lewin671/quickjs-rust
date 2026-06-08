// Derived from: test/annexB/built-ins/RegExp/prototype/Symbol.split/Symbol.match-getter-recompiles-source.js
var regExp = /a/;
Object.defineProperty(regExp, Symbol.match, {
  get: function() {
    regExp.compile("b");
  }
});

var result = regExp[Symbol.split]("abba");

if (result.length !== 3) { throw "expected three split segments"; }
if (result[0] !== "a") { throw "expected first segment from recompiled source"; }
if (result[1] !== "") { throw "expected empty middle segment"; }
if (result[2] !== "a") { throw "expected final segment from recompiled source"; }
