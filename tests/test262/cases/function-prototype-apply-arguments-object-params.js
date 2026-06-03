// Derived from: test/built-ins/Function/prototype/apply/S15.3.4.3_A7_T4.js
var i = 0;
var p = {
  toString: function() {
    return "a" + (++i);
  }
};

var obj = {};
new Function(p, p, p, "this.shifted = a3;").apply(obj, (function() {
  return arguments;
})("a", "b", "c"));

if (obj.shifted !== "c") { throw; }
if (typeof this.shifted !== "undefined") { throw; }
