// Derived from: test/language/statements/variable/S12.2_A3.js
function f() {
  if (x !== undefined) { throw; }
  var x = 2;
  return x;
}

if (f() !== 2) { throw; }
