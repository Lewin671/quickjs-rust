// Derived from: test/language/statements/try/S12.14_A6.js
var count = 0;

try {
  count += 1;
} finally {
  count *= 2;
}

if (count !== 2) { throw; }
