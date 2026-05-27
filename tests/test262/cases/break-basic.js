// Derived from: test/language/statements/break/12.8-1.js
var sum = 0;
for (var i = 1; i <= 10; i++) {
  if (i === 6) {
    break
    ;
  }
  sum += i;
}
if (sum !== 15) { throw; }
