// Derived from: test/language/statements/do-while/S12.6.1_A1.js
var seen;

do seen = 1; while (false);
if (seen !== 1) {
  throw;
}

do seen = 2; while (0);
if (seen !== 2) {
  throw;
}

do seen = 3; while ("");
if (seen !== 3) {
  throw;
}
