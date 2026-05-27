// Derived from: test/language/expressions/call/S11.2.4_A1.1_T1.js
function f_arg() {
  return arguments;
}

if (f_arg().length !== 0) { throw; }
if (f_arg()[0] !== undefined) { throw; }
