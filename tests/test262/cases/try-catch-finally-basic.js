// Derived from: test/language/statements/try/S12.14_A6.js
var count = 0;

try {
  count = 1;
  throw "expected";
} catch (error) {
  if (error !== "expected") { throw; }
  count *= 2;
} finally {
  count += 1;
}

if (count !== 3) { throw; }
