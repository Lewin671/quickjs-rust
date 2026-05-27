// Derived from: test/language/statements/for/S12.6.3_A14.js
for (var i = 0; i < 10; i++) {}
if (i !== 10) { throw; }

var j = 0;
for (var k = 16; k > 1; k = k / 2) {
  j++;
}
if (k !== 1) { throw; }
if (j !== 4) { throw; }
