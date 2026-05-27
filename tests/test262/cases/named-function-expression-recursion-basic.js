// Derived from: test/language/statements/function/S13_A3_T1.js
var factorial = function fact(n) {
  return n <= 1 ? 1 : n * fact(n - 1);
};

if (factorial(5) !== 120) { throw; }
