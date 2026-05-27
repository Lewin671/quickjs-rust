// Derived from: test/language/expressions/call/spread-mult-empty.js
var value = (function(input) {
  return input + 1;
})(2);

if (value !== 3) { throw; }
