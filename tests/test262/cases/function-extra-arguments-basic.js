// Derived from: test/language/expressions/call/S11.2.4_A1.4_T3.js
function f_arg(x, y, z) {
  return arguments[3];
}

if (f_arg(1, 2, 3, 4) !== 4) { throw; }
