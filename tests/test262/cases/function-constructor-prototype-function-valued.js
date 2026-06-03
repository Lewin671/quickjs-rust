// Derived from: test/built-ins/Function/prototype/apply/S15.3.4.3_A1_T1.js
var proto = Function();
proto.value = 12;

function Factory() {}
Factory.prototype = proto;

var obj = new Factory();
if (typeof obj.apply !== "function") { throw; }
if (obj.value !== 12) { throw; }
