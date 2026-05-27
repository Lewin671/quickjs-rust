// Derived from: test/language/expressions/function/scope-name-var-close.js
var f = function hidden() {
  return 1;
};

if (typeof hidden !== "undefined") { throw; }
if (f() !== 1) { throw; }
