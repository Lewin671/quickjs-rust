// Derived from: test/language/expressions/instanceof/prototype-getter-with-object.js
function C() {}
C.prototype = { tag: 1 };
var instance = new C();

if (!(instance instanceof C)) { throw; }
