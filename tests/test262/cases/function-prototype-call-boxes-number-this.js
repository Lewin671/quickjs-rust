// Derived from: test/built-ins/Function/prototype/call/S15.3.4.4_A5_T1.js
var obj = 1;

var retobj = Function("this.touched = true; return this;").call(obj);

if (typeof obj.touched !== "undefined") { throw; }
if (!retobj["touched"]) { throw; }
