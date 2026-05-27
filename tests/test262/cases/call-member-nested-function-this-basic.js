// Derived from: test/language/expressions/this/S11.1.1_A3.1.js
function returnThis() {
  return this;
}

function callReturnThis() {
  return returnThis();
}

var object = {};
object.callReturnThis = callReturnThis;

if (object.callReturnThis() !== this) { throw; }
