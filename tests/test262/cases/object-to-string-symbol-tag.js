// Derived from: test/built-ins/Object/prototype/toString/symbol-tag-str.js
// Derived from: test/built-ins/Object/prototype/toString/symbol-tag-non-str.js
var custom = {};
custom[Symbol.toStringTag] = "test262";

if (Object.prototype.toString.call(custom) !== "[object test262]") {
  throw "expected string Symbol.toStringTag to override builtin tag";
}

custom[Symbol.toStringTag] = 86;
if (Object.prototype.toString.call(custom) !== "[object Object]") {
  throw "expected non-string Symbol.toStringTag to be ignored";
}
