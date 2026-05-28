// Derived from: test/built-ins/Function/prototype/call/S15.3.4.4_A5_T5.js
function add(a, b) {
  return this.base + a + b;
}

var context = { base: 4 };
if (add.call(context, 2, 3) !== 9) { throw; }
