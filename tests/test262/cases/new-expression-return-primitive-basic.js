// Derived from: test/language/expressions/new/S11.2.2_A4_T1.js
function Box() {
  this.value = 6;
  return 1;
}

var box = new Box();
if (box.value !== 6) { throw; }
