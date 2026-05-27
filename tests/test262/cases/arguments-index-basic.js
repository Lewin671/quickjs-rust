// Derived from: test/language/statements/function/S13.2_A2_T1.js
function f_arg() {
  return arguments[0];
}

if (f_arg("jedi") !== "jedi") { throw; }
