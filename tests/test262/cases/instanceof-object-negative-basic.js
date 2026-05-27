// Derived from: test/language/expressions/instanceof/S11.8.6_A4_T1.js
function C() {}
function D() {}
var instance = new C();

if (instance instanceof D) { throw; }
