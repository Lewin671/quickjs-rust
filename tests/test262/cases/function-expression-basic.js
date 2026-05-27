// Derived from: test/language/expressions/call/scope-var-open.js
var add = function(left, right) {
  return left + right;
};

if (add(2, 3) !== 5) { throw; }
