// Derived from: test/language/statements/function/S13_A2_T2.js
function outer() {
  return inner();

  function inner() {
    return "hoisted";
  }
}

if (outer() !== "hoisted") { throw; }
