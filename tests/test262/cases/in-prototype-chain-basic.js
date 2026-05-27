// Derived from: test/language/expressions/instanceof/prototype-getter-with-object.js
function C() {}
C.prototype.value = 4;

var instance = new C();
if (!("value" in instance)) { throw; }
