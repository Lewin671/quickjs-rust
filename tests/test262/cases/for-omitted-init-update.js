// Derived from: test/language/statements/for/head-init-var-check-empty-inc-empty-syntax.js
var count = 0;
for (; count < 5; ) {
  count = count + 1;
}
if (count !== 5) { throw; }
