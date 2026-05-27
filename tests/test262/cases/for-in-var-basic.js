// Derived from: test/language/statements/for-in/order-simple-object.js
var count = 0;

for (var key in { a: 1, b: 2 }) {
  count++;
}

if (count !== 2) {
  throw;
}
