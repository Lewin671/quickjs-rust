// Derived from: test/built-ins/Function/prototype/S15.3.3.1_A1.js
function C() {
  this.value = 9;
}
C.prototype.value = 4;

var instance = new C();
if (instance.value !== 9) { throw; }
