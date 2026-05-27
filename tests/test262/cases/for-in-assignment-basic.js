// Derived from: test/language/statements/for-in/cptn-expr-itr.js
var key;
var count = 0;

for (key in { x: 0 }) {
  count++;
}

if (key !== "x") {
  throw;
}

if (count !== 1) {
  throw;
}
