// Derived from: test/language/statements/variable/S12.2_A2.js
if (false) {
  var x = 1;
}

if (x !== undefined) { throw; }
