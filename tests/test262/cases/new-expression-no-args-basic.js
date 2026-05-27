// Derived from: test/language/expressions/new/S11.2.2_A1.1.js
function Empty() {
  this.value = 9;
}

var empty = new Empty;
if (empty.value !== 9) { throw; }
