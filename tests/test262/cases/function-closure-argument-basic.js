// Derived from: test/language/statements/function/S13.2.1_A5_T2.js
function bind(left) {
  return function(right) {
    return left + right;
  };
}

if (bind(2)(3) !== 5) { throw; }
