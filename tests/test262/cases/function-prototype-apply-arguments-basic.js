// Derived from: test/built-ins/Function/prototype/apply/S15.3.4.3_A7_T5.js
function add(a, b, c) {
  return a + b + c;
}

function caller() {
  return add.apply(null, arguments);
}

if (caller("", 1, 2) !== "12") { throw; }
