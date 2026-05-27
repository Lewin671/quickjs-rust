// Derived from: test/language/expressions/call/S11.2.4_A1.1_T2.js
function f_arg(x, y) {
  return y;
}

if (f_arg(1) !== undefined) { throw; }
