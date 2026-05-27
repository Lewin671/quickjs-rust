// Derived from: test/language/statements/continue/12.7-1.js
var sum = 0;
for (var i = 1; i <= 10; i++) {
  if (true) continue
  ; else {}
  sum += i;
}
if (sum !== 0) { throw; }
