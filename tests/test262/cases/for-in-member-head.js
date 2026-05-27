// Derived from: test/language/statements/for-in/head-lhs-member.js
var iterCount = 0;
var x = {};

for (x.y in { attr: null }) {
  if (x.y !== "attr") {
    throw;
  }
  iterCount++;
}

if (iterCount !== 1) {
  throw;
}
