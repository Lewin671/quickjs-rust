// Derived from: test/language/statements/function/S13_A3_T1.js
var f = function named() {
  return typeof named;
};

if (f() !== "function") { throw; }
