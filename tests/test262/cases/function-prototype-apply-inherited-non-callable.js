// Derived from: test/built-ins/Function/prototype/apply/S15.3.4.3_A1_T1.js
var proto = Function();

function Factory() {}
Factory.prototype = proto;

var obj = new Factory();
if (typeof obj.apply !== "function") { throw; }

var caught = false;
try {
  obj.apply();
} catch (error) {
  caught = error instanceof TypeError;
}

if (!caught) { throw; }
