// Derived from: test/language/expressions/new/S11.2.2_A4_T1.js
function Box() {
  this.value = 1;
  return { value: 4 };
}

var box = new Box();
if (box.value !== 4) { throw; }
