// Derived from: test/built-ins/Function/prototype/apply/S15.3.4.3_A7_T1.js
function add(a, b) {
  return this.base + a + b;
}

var context = { base: 4 };
if (add.apply(context, [2, 3]) !== 9) { throw; }
