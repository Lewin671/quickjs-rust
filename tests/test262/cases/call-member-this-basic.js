// Derived from: test/language/expressions/call/with-base-obj.js
function captureThis() {
  return this;
}

var object = {};
object.captureThis = captureThis;

if (object.captureThis() !== object) { throw; }
