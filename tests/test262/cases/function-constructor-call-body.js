// Derived from: test/built-ins/Function/S15.3.5_A2_T2.js
var f = new Function("arg1,arg2", "var x = arg1; this.y = arg2; return arg1 + arg2;");

if (f("1", 2) !== "12") { throw; }
if (this.y !== 2) { throw; }
