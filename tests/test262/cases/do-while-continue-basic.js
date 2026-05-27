// Derived from: test/language/statements/do-while/S12.6.1_A8.js
var i = 0;
var seen = 0;

do {
  i++;
  if (i === 2) {
    continue;
  }
  seen++;
} while (i < 3);

if (i !== 3) {
  throw;
}

if (seen !== 2) {
  throw;
}
